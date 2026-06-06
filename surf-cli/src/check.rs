//! `surf check` — the gate (§9.1.3, §5). For each anchored span: resolve, AST-canonical
//! hash, compare to the stored hash. Any divergence blocks (non-zero exit). `--format json`
//! emits the structured report that optional layers attach to; the verdict itself is
//! deterministic and needs no git. `old_code`/`magnitude` are recovered best-effort from the
//! previous source via `git show <base>:<path>` and are advisory only.

use crate::workspace::{read_site, Workspace};
use anyhow::{Context, Result};
use clap::ValueEnum;
use std::path::Path;
use std::process::{Command, ExitCode};
use surf_core::{diff_magnitude, hash_anchor, parse_hub, resolve, Divergence, DivergenceKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Human,
    Json,
}

pub fn run(ws: &Workspace, format: Format, base: &str) -> Result<ExitCode> {
    let divergences = check_workspace(ws, base)?;

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

fn check_workspace(ws: &Workspace, base: &str) -> Result<Vec<Divergence>> {
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
            if let Some(d) = check_claim(ws, &rel, claim, base) {
                out.push(d);
            }
        }
    }
    Ok(out)
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

        assert!(check_workspace(&ws_at(root.to_path_buf()), "HEAD")
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

        assert!(check_workspace(&ws_at(root.to_path_buf()), "HEAD")
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

        let d = check_workspace(&ws_at(root.to_path_buf()), "HEAD").unwrap();
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

        let d = check_workspace(&ws_at(root.to_path_buf()), "HEAD").unwrap();
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

        let d = check_workspace(&ws_at(root.to_path_buf()), "HEAD").unwrap();
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

        let d = check_workspace(&ws_at(root.to_path_buf()), "HEAD").unwrap();
        let json = serde_json::to_value(&d).unwrap();
        let obj = json[0].as_object().unwrap();
        for key in [
            "hub", "claim", "at", "kind", "old_hash", "new_hash", "new_code", "prose",
        ] {
            assert!(obj.contains_key(key), "missing key `{key}` in {obj:?}");
        }
    }
}
