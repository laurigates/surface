//! `surf check` — the gate (§9.1.3, §5). For each anchored span: resolve, AST-canonical
//! hash, compare to the stored hash. Any divergence blocks (non-zero exit). `--format json`
//! emits the structured report that optional layers attach to; the verdict itself is
//! deterministic and needs no git. `old_code`/`magnitude` are recovered best-effort from the
//! previous source via `git show <base>:<path>` and are advisory only.

use crate::format::Format;
use crate::workspace::{read_site, Workspace};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;
use std::process::{Command, ExitCode};
use surf_core::{
    diff_magnitude, hash_anchor, parse_anchor, parse_hub, resolve, Divergence, DivergenceKind,
};

pub fn run(
    ws: &Workspace,
    format: Format,
    base: Option<&str>,
    files: &[String],
) -> Result<ExitCode> {
    let divergences = check_workspace(ws, base, files)?;

    match format {
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&divergences)?);
        }
        Format::Human => print_human(&divergences),
    }

    Ok(if divergences.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn check_workspace(
    ws: &Workspace,
    base: Option<&str>,
    files: &[String],
) -> Result<Vec<Divergence>> {
    let scope = Scope::build(ws, base, files);
    // Enrichment always needs a ref; an explicit --base doubles as the diff base, else HEAD.
    let enrich_base = base.unwrap_or("HEAD");

    let mut out = Vec::new();
    for hub_path in ws.hub_paths()? {
        let rel = hub_path
            .strip_prefix(&ws.root)
            .unwrap_or(&hub_path)
            .display()
            .to_string();
        let content = std::fs::read_to_string(&hub_path)
            .with_context(|| format!("reading {}", hub_path.display()))?;
        let Ok(hub) = parse_hub(&content) else {
            // Malformed hubs are lint's job; check skips them rather than miscounting.
            continue;
        };

        for claim in &hub.frontmatter.anchors {
            if !scope.includes(claim) {
                continue;
            }
            if let Some(d) = check_claim(ws, &rel, claim, enrich_base) {
                out.push(d);
            }
        }
    }
    Ok(out)
}

/// Which claims `check` evaluates. Each active filter narrows the set; a claim must satisfy
/// every active filter (intersection). With neither filter active, every claim is in scope.
struct Scope {
    changed: Option<HashSet<String>>,
    globs: Vec<glob::Pattern>,
}

impl Scope {
    fn build(ws: &Workspace, base: Option<&str>, files: &[String]) -> Scope {
        // A bad ref / non-repo yields None — we fall back to a full check rather than
        // silently checking nothing.
        let changed = base.and_then(|b| git_changed_files(&ws.root, b));
        let globs = files
            .iter()
            .filter_map(|p| glob::Pattern::new(p).ok())
            .collect();
        Scope { changed, globs }
    }

    fn includes(&self, claim: &surf_core::Claim) -> bool {
        let anchor_files: Vec<String> = claim
            .at
            .sites()
            .iter()
            .filter_map(|s| parse_anchor(s).ok().map(|a| a.file))
            .collect();

        if let Some(changed) = &self.changed {
            if !anchor_files.iter().any(|f| changed.contains(f)) {
                return false;
            }
        }
        if !self.globs.is_empty()
            && !anchor_files
                .iter()
                .any(|f| self.globs.iter().any(|g| g.matches(f)))
        {
            return false;
        }
        true
    }
}

fn check_claim(
    ws: &Workspace,
    hub: &str,
    claim: &surf_core::Claim,
    base: &str,
) -> Option<Divergence> {
    let prose = claim.claim.trim().to_string();
    let sites = claim.at.sites();
    let at_display = sites.join("  +  ");
    let single = sites.len() == 1;

    let mk = |kind, old_hash, new_hash, old_code, new_code, magnitude, detail| {
        Some(Divergence {
            hub: hub.to_string(),
            claim: prose.clone(),
            at: at_display.clone(),
            kind,
            old_hash,
            new_hash,
            old_code,
            new_code,
            prose: prose.clone(),
            magnitude,
            detail,
        })
    };
    let unresolvable = |detail: String| {
        mk(
            DivergenceKind::Unresolvable,
            claim.hash.clone(),
            None,
            None,
            None,
            None,
            Some(detail),
        )
    };

    // Resolve and hash every site; the claim's hash is the combination (§6.3).
    let mut site_hashes = Vec::with_capacity(sites.len());
    let mut first_new_code = None;
    for site in sites {
        let (current, lang, anchor) = match read_site(ws, site) {
            Ok(parts) => parts,
            Err(e) => return unresolvable(e.to_string()),
        };
        let span = match resolve(&current, lang, &anchor) {
            Ok(span) => span,
            Err(e) => return unresolvable(e.to_string()),
        };
        if single {
            first_new_code = current
                .get(span.start_byte..span.end_byte)
                .map(str::to_string);
        }
        site_hashes.push(hash_anchor(&current, lang, &anchor).ok()?);
    }
    let new_hash = surf_core::combine_site_hashes(&site_hashes);

    match &claim.hash {
        None => mk(
            DivergenceKind::Unverified,
            None,
            Some(new_hash),
            None,
            first_new_code,
            None,
            None,
        ),
        Some(stored) if *stored == new_hash => None, // clean
        Some(stored) => {
            // Best-effort old_code + magnitude from git, for single-site anchors only.
            let (old_code, magnitude) = if single {
                enrich_from_git(ws, base, &sites[0])
            } else {
                (None, None)
            };
            mk(
                DivergenceKind::Changed,
                Some(stored.clone()),
                Some(new_hash),
                old_code,
                first_new_code,
                magnitude,
                None,
            )
        }
    }
}

fn enrich_from_git(
    ws: &Workspace,
    base: &str,
    site: &str,
) -> (Option<String>, Option<surf_core::Magnitude>) {
    let Ok((current, lang, anchor)) = read_site(ws, site) else {
        return (None, None);
    };
    let Some(old_source) = git_show(&ws.root, base, &anchor.file) else {
        return (None, None);
    };
    let old_code = resolve(&old_source, lang, &anchor).ok().and_then(|sp| {
        old_source
            .get(sp.start_byte..sp.end_byte)
            .map(str::to_string)
    });
    let magnitude = diff_magnitude(&old_source, &current, lang, &anchor).ok();
    (old_code, magnitude)
}

/// Files changed between the merge base of `base`..HEAD and the working tree. Paths are
/// repo-root-relative; they match `Anchor.file` (workspace-root-relative) when the
/// workspace root is the repo root, the normal case. `None` if git can't answer.
fn git_changed_files(root: &Path, base: &str) -> Option<HashSet<String>> {
    let merge_base = Command::new("git")
        .current_dir(root)
        .args(["merge-base", base, "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        // Shallow clones may lack the merge base; diff against the ref directly.
        .unwrap_or_else(|| base.to_string());

    let output = Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-only", &merge_base])
        .output()
        .ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_string)
            .collect()
    })
}

fn git_show(root: &Path, base: &str, rel_file: &str) -> Option<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["show", &format!("{base}:{rel_file}")])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn print_human(divergences: &[Divergence]) {
    for d in divergences {
        let (tag, hint) = match d.kind {
            DivergenceKind::Changed => ("DIVERGED", None),
            DivergenceKind::Unverified => ("UNVERIFIED", Some("run `surf verify`")),
            DivergenceKind::Unresolvable => ("UNRESOLVED", Some("run `surf lint`")),
        };
        println!("{tag}  {} :: {}", d.hub, d.at);
        if let Some(detail) = &d.detail {
            println!("    {detail}");
        }
        if let (Some(old), Some(new)) = (&d.old_hash, &d.new_hash) {
            let mag = d
                .magnitude
                .map(|m| format!("  (magnitude: {m:?})"))
                .unwrap_or_default();
            println!("    stored {old} → now {new}{mag}");
        }
        if let Some(hint) = hint {
            println!("    {hint}");
        }
        println!("    claim: {}", d.prose);
    }

    if divergences.is_empty() {
        println!("surf check: all anchored spans match their stored hashes.");
    } else {
        println!("surf check: {} divergence(s).", divergences.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use surf_core::{parse_anchor, Lang};

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, content).unwrap();
    }

    fn ws_at(root: PathBuf) -> Workspace {
        Workspace::discover(&root).unwrap()
    }

    fn stored_hash(src: &str, anchor: &str) -> String {
        hash_anchor(src, Lang::Rust, &parse_anchor(anchor).unwrap()).unwrap()
    }

    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(root)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    #[test]
    fn clean_when_hash_matches() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let src = "pub fn add(a: i64, b: i64) -> i64 { a + b }\n";
        write(root, "surf.toml", "");
        write(root, "src/m.rs", src);
        let h = stored_hash(src, "src/m.rs > add");
        write(
            root,
            "hubs/a.md",
            &format!("---\nsummary: x\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n    hash: {h}\n---\n"),
        );

        assert!(check_workspace(&ws_at(root.to_path_buf()), None, &[])
            .unwrap()
            .is_empty());
    }

    #[test]
    fn per_symbol_not_per_file() {
        // Anchor `add`; modify the *other* function in the same file. Must stay clean.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let original = "pub fn add(a: i64, b: i64) -> i64 { a + b }\npub fn other() -> i64 { 1 }\n";
        let h = stored_hash(original, "src/m.rs > add");
        write(root, "surf.toml", "");
        write(
            root,
            "src/m.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a + b }\npub fn other() -> i64 { 999 }\n",
        );
        write(
            root,
            "hubs/a.md",
            &format!("---\nsummary: x\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n    hash: {h}\n---\n"),
        );

        assert!(check_workspace(&ws_at(root.to_path_buf()), None, &[])
            .unwrap()
            .is_empty());
    }

    #[test]
    fn unverified_when_no_stored_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "surf.toml", "");
        write(root, "src/m.rs", "pub fn add() -> i64 { 1 }\n");
        write(
            root,
            "hubs/a.md",
            "---\nsummary: x\nanchors:\n  - claim: c\n    at: src/m.rs > add\n---\n",
        );

        let d = check_workspace(&ws_at(root.to_path_buf()), None, &[]).unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, DivergenceKind::Unverified);
    }

    #[test]
    fn changed_span_diverges_with_old_code_and_magnitude_from_git() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let v1 = "pub fn add(a: i64, b: i64) -> i64 { a + b }\n";
        let h = stored_hash(v1, "src/m.rs > add");
        write(root, "surf.toml", "");
        write(root, "src/m.rs", v1);
        write(
            root,
            "hubs/a.md",
            &format!("---\nsummary: x\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n    hash: {h}\n---\n"),
        );

        git(root, &["init", "-q"]);
        git(
            root,
            &["-c", "user.email=t@t", "-c", "user.name=t", "add", "."],
        );
        git(
            root,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "-q",
                "-m",
                "v1",
            ],
        );

        // Working-tree change: flip the operator.
        write(
            root,
            "src/m.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a - b }\n",
        );

        let d = check_workspace(&ws_at(root.to_path_buf()), None, &[]).unwrap();
        assert_eq!(d.len(), 1);
        let d = &d[0];
        assert_eq!(d.kind, DivergenceKind::Changed);
        assert_eq!(d.old_hash.as_deref(), Some(h.as_str()));
        assert!(d.new_hash.is_some() && d.new_hash != d.old_hash);
        assert!(d.old_code.as_deref().unwrap().contains("a + b"));
        assert!(d.new_code.as_deref().unwrap().contains("a - b"));
        assert!(d.magnitude.is_some());
    }

    #[test]
    fn unsupported_file_type_is_unresolvable_with_detail() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "surf.toml", "");
        write(root, "schema.sql", "CREATE TABLE users (id int);\n");
        write(
            root,
            "hubs/a.md",
            "---\nsummary: x\nanchors:\n  - claim: c\n    at: schema.sql > users\n---\n",
        );

        let d = check_workspace(&ws_at(root.to_path_buf()), None, &[]).unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, DivergenceKind::Unresolvable);
        assert_eq!(
            d[0].detail.as_deref(),
            Some("unsupported file type: schema.sql")
        );
    }

    #[test]
    fn json_contract_has_expected_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let v1 = "pub fn add(a: i64, b: i64) -> i64 { a + b }\n";
        let h = stored_hash(v1, "src/m.rs > add");
        write(root, "surf.toml", "");
        write(
            root,
            "src/m.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a - b }\n",
        );
        write(
            root,
            "hubs/a.md",
            &format!("---\nsummary: x\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n    hash: {h}\n---\n"),
        );

        let d = check_workspace(&ws_at(root.to_path_buf()), None, &[]).unwrap();
        let json = serde_json::to_value(&d).unwrap();
        let obj = json[0].as_object().unwrap();
        for key in [
            "hub", "claim", "at", "kind", "old_hash", "new_hash", "new_code", "prose",
        ] {
            assert!(obj.contains_key(key), "missing key `{key}` in {obj:?}");
        }
    }

    /// Two diverged claims in different files; both surface with no scope, but a `--files`
    /// glob narrows the result to the matching file.
    fn two_diverged_files(root: &Path) {
        let a = "pub fn add(a: i64, b: i64) -> i64 { a + b }\n";
        let s = "pub fn sub(a: i64, b: i64) -> i64 { a - b }\n";
        let ha = stored_hash(a, "src/a.rs > add");
        let hs = stored_hash(s, "src/b.rs > sub");
        write(root, "surf.toml", "");
        // Working tree diverges from the stored hashes.
        write(
            root,
            "src/a.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a - b }\n",
        );
        write(
            root,
            "src/b.rs",
            "pub fn sub(a: i64, b: i64) -> i64 { a + b }\n",
        );
        write(
            root,
            "hubs/a.md",
            &format!(
                "---\nsummary: x\nanchors:\n  - claim: add\n    at: src/a.rs > add\n    hash: {ha}\n  - claim: sub\n    at: src/b.rs > sub\n    hash: {hs}\n---\n"
            ),
        );
    }

    #[test]
    fn files_scope_limits_claims() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        two_diverged_files(root);
        let ws = ws_at(root.to_path_buf());

        assert_eq!(check_workspace(&ws, None, &[]).unwrap().len(), 2);

        let scoped = check_workspace(&ws, None, &["src/a.rs".to_string()]).unwrap();
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].at, "src/a.rs > add");

        let globbed = check_workspace(&ws, None, &["src/b*.rs".to_string()]).unwrap();
        assert_eq!(globbed.len(), 1);
        assert_eq!(globbed[0].at, "src/b.rs > sub");
    }

    #[test]
    fn base_scope_limits_to_changed() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        two_diverged_files(root);

        // Commit the diverged working tree as v0, then change only src/a.rs.
        git(root, &["init", "-q"]);
        git(
            root,
            &["-c", "user.email=t@t", "-c", "user.name=t", "add", "."],
        );
        git(
            root,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "-q",
                "-m",
                "v0",
            ],
        );
        write(
            root,
            "src/a.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a * b }\n",
        );

        let ws = ws_at(root.to_path_buf());
        let scoped = check_workspace(&ws, Some("HEAD"), &[]).unwrap();
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].at, "src/a.rs > add");
    }

    #[test]
    fn no_flags_checks_everything() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        two_diverged_files(root);
        let ws = ws_at(root.to_path_buf());
        assert_eq!(check_workspace(&ws, None, &[]).unwrap().len(), 2);
    }
}
