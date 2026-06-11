//! `surf suggest <globs>` — propose anchors for public symbols no hub covers yet (§8, #18).
//! Scans the given source files, lists each public function and method that isn't already
//! anchored, and prints a copy-pasteable starter hub. With `--all` it also proposes the
//! non-callable targets `resolve` accepts (classes, constants, type aliases, class attributes),
//! so they're discoverable. Suggestions only: it never writes a file and never stamps a hash —
//! the author edits the claims and runs `surf verify`. A glob that matches no files is reported
//! (and fails when every glob is empty) so typos don't look clean.

use crate::format::Format;
use crate::workspace::Workspace;
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashSet;
use std::process::ExitCode;
use surf_core::{parse_anchor, public_symbols, Lang, Surface};

#[derive(Debug, Clone, Serialize)]
struct Suggestion {
    file: String,
    symbol: String,
    at: String,
}

/// Per-glob tally so a typo'd glob (zero files) reads differently from a clean "all anchored".
struct GlobReport {
    pattern: String,
    files_matched: usize,
    supported_matched: usize,
}

struct ScanResult {
    suggestions: Vec<Suggestion>,
    globs: Vec<GlobReport>,
}

pub fn run(ws: &Workspace, globs: &[String], all: bool, format: Format) -> Result<ExitCode> {
    let surface = if all { Surface::All } else { Surface::Callables };
    let covered = covered_anchors(ws)?;
    let result = scan(ws, globs, &covered, surface)?;

    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&result.suggestions)?),
        Format::Human => print_human(&result.suggestions),
    }
    // Warnings go to stderr so JSON stdout stays machine-parseable.
    for g in &result.globs {
        if g.files_matched == 0 {
            eprintln!("surf suggest: glob \"{}\" matched no files.", g.pattern);
        } else if g.supported_matched == 0 {
            eprintln!(
                "surf suggest: glob \"{}\" matched files, but none in a supported language.",
                g.pattern
            );
        }
    }

    // A typo'd glob should fail scripts/CI; but only when *every* glob matched nothing, so a
    // partially-correct invocation still succeeds.
    let all_empty = !result.globs.is_empty() && result.globs.iter().all(|g| g.files_matched == 0);
    Ok(if all_empty {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// Full anchor paths already covered by some hub, normalized to `file > seg > seg` (positional
/// `@N` dropped). Keyed on the whole path so anchoring one method doesn't hide its siblings.
fn covered_anchors(ws: &Workspace) -> Result<HashSet<String>> {
    let mut covered = HashSet::new();
    for loaded in ws.iter_hubs()? {
        let Ok(hub) = loaded.hub else {
            continue;
        };
        for claim in &hub.frontmatter.anchors {
            for site in claim.at.sites() {
                if let Ok(anchor) = parse_anchor(site) {
                    let path: Vec<&str> = anchor.segments.iter().map(|s| s.name.as_str()).collect();
                    covered.insert(format!("{} > {}", anchor.file, path.join(" > ")));
                }
            }
        }
    }
    Ok(covered)
}

fn scan(
    ws: &Workspace,
    globs: &[String],
    covered: &HashSet<String>,
    surface: Surface,
) -> Result<ScanResult> {
    let mut out = Vec::new();
    let mut reports = Vec::new();
    for pattern in globs {
        let joined = ws.root.join(pattern);
        let glob_str = joined
            .to_str()
            .with_context(|| format!("glob is not valid UTF-8: {}", joined.display()))?;
        let mut files_matched = 0;
        let mut supported_matched = 0;
        for entry in glob::glob(glob_str).context("invalid glob pattern")? {
            let path = entry?;
            if !path.is_file() {
                continue;
            }
            files_matched += 1;
            let rel = path
                .strip_prefix(&ws.root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            let Some(lang) = Lang::from_path(&rel) else {
                continue;
            };
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            supported_matched += 1;
            for segments in public_symbols(&source, lang, surface) {
                let at = format!("{rel} > {}", segments.join(" > "));
                if covered.contains(&at) {
                    continue;
                }
                let symbol = segments.last().cloned().unwrap_or_default();
                out.push(Suggestion {
                    file: rel.clone(),
                    symbol,
                    at,
                });
            }
        }
        reports.push(GlobReport {
            pattern: pattern.clone(),
            files_matched,
            supported_matched,
        });
    }
    out.sort_by(|a, b| a.at.cmp(&b.at));
    out.dedup_by(|a, b| a.at == b.at);
    Ok(ScanResult {
        suggestions: out,
        globs: reports,
    })
}

fn print_human(suggestions: &[Suggestion]) {
    if suggestions.is_empty() {
        println!("surf suggest: no unanchored public symbols found.");
        return;
    }
    println!(
        "# {} unanchored public symbol(s). Paste into a hub (or `surf new <name>`), write the",
        suggestions.len()
    );
    println!(
        "# claims, then `surf verify`. These are suggestions — nothing was written or stamped."
    );
    println!("---");
    println!("summary: TODO one-line summary of this domain.");
    println!("anchors:");
    for s in suggestions {
        println!(
            "  - claim: TODO the invariant {} guarantees, in prose",
            s.symbol
        );
        println!("    at: {}", s.at);
    }
    println!("refs: []");
    println!("---");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn ws_with(files: &[(&str, &str)]) -> (tempfile::TempDir, Workspace) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("surf.toml"), "").unwrap();
        fs::create_dir_all(root.join("hubs")).unwrap();
        for (rel, content) in files {
            let p = root.join(rel);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(p, content).unwrap();
        }
        let ws = Workspace::discover(root).unwrap();
        (tmp, ws)
    }

    #[test]
    fn suggests_only_unanchored_public_fns() {
        let (_t, ws) = ws_with(&[
            ("src/m.rs", "pub fn a() {}\npub fn b() {}\nfn c() {}\n"),
            (
                "hubs/a.md",
                "---\nsummary: x\nanchors:\n  - claim: a does\n    at: src/m.rs > a\n---\n",
            ),
        ]);
        let covered = covered_anchors(&ws).unwrap();
        let s = scan(&ws, &["src/*.rs".to_string()], &covered, Surface::Callables)
            .unwrap()
            .suggestions;
        // `a` is anchored, `c` is private — only `b` remains.
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].at, "src/m.rs > b");
        assert_eq!(s[0].symbol, "b");
    }

    #[test]
    fn unsupported_files_are_skipped() {
        let (_t, ws) = ws_with(&[("notes.txt", "pub fn a() {}\n")]);
        let covered = covered_anchors(&ws).unwrap();
        let s = scan(&ws, &["*.txt".to_string()], &covered, Surface::Callables)
            .unwrap()
            .suggestions;
        assert!(s.is_empty());
    }

    #[test]
    fn json_shape_has_no_hash() {
        let (_t, ws) = ws_with(&[("src/m.rs", "pub fn solo() {}\n")]);
        let covered = covered_anchors(&ws).unwrap();
        let s = scan(&ws, &["src/*.rs".to_string()], &covered, Surface::Callables)
            .unwrap()
            .suggestions;
        let json = serde_json::to_value(&s).unwrap();
        let obj = json[0].as_object().unwrap();
        for key in ["file", "symbol", "at"] {
            assert!(obj.contains_key(key), "missing `{key}` in {obj:?}");
        }
        assert!(!obj.contains_key("hash"));
    }

    #[test]
    fn proposes_python_methods_as_class_method_anchors() {
        let (_t, ws) = ws_with(&[(
            "src/api.py",
            "class Client:\n    def fetch(self):\n        pass\n    def send(self):\n        pass\n",
        )]);
        let covered = covered_anchors(&ws).unwrap();
        let s = scan(&ws, &["src/*.py".to_string()], &covered, Surface::Callables)
            .unwrap()
            .suggestions;
        let ats: Vec<&str> = s.iter().map(|x| x.at.as_str()).collect();
        assert_eq!(
            ats,
            vec!["src/api.py > Client > fetch", "src/api.py > Client > send"]
        );
    }

    #[test]
    fn all_flag_proposes_classes_and_non_callables() {
        // Default stays callables; --all surfaces the class, constant, and class attribute too,
        // so the kinds `resolve` accepts are discoverable (#52).
        let (_t, ws) = ws_with(&[(
            "src/api.py",
            "CONST = 1\n\nclass Client:\n    timeout: int = 30\n    def fetch(self):\n        pass\n",
        )]);
        let covered = covered_anchors(&ws).unwrap();
        let default = scan(&ws, &["src/*.py".to_string()], &covered, Surface::Callables)
            .unwrap()
            .suggestions;
        let default_ats: Vec<&str> = default.iter().map(|x| x.at.as_str()).collect();
        assert_eq!(default_ats, vec!["src/api.py > Client > fetch"]);

        let all = scan(&ws, &["src/*.py".to_string()], &covered, Surface::All)
            .unwrap()
            .suggestions;
        let all_ats: Vec<&str> = all.iter().map(|x| x.at.as_str()).collect();
        assert_eq!(
            all_ats,
            vec![
                "src/api.py > CONST",
                "src/api.py > Client",
                "src/api.py > Client > fetch",
                "src/api.py > Client > timeout",
            ]
        );
    }

    #[test]
    fn anchoring_one_method_does_not_hide_siblings() {
        let (_t, ws) = ws_with(&[
            (
                "src/api.py",
                "class Client:\n    def fetch(self):\n        pass\n    def send(self):\n        pass\n",
            ),
            (
                "hubs/a.md",
                "---\nsummary: x\nanchors:\n  - claim: c\n    at: src/api.py > Client > fetch\n---\n",
            ),
        ]);
        let covered = covered_anchors(&ws).unwrap();
        let s = scan(&ws, &["src/*.py".to_string()], &covered, Surface::Callables)
            .unwrap()
            .suggestions;
        // `fetch` is anchored; only `send` should remain.
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].at, "src/api.py > Client > send");
    }

    #[test]
    fn zero_match_glob_is_reported_and_fails_alone() {
        let (_t, ws) = ws_with(&[("src/m.rs", "pub fn a() {}\n")]);
        let r = scan(
            &ws,
            &["zzz/nope/**/*.go".to_string()],
            &covered_anchors(&ws).unwrap(),
            Surface::Callables,
        )
        .unwrap();
        assert_eq!(r.globs.len(), 1);
        assert_eq!(r.globs[0].files_matched, 0);
        assert!(r.suggestions.is_empty());
        let code = run(&ws, &["zzz/nope/**/*.go".to_string()], false, Format::Human).unwrap();
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::FAILURE));
    }

    #[test]
    fn partial_match_still_succeeds() {
        let (_t, ws) = ws_with(&[("src/m.rs", "pub fn a() {}\n")]);
        let code = run(
            &ws,
            &["src/*.rs".to_string(), "zzz/nope/*.go".to_string()],
            false,
            Format::Human,
        )
        .unwrap();
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
    }

    #[test]
    fn unsupported_language_match_is_distinguished() {
        let (_t, ws) = ws_with(&[("notes.txt", "hello\n")]);
        let r = scan(&ws, &["*.txt".to_string()], &covered_anchors(&ws).unwrap(), Surface::Callables).unwrap();
        assert_eq!(r.globs[0].files_matched, 1);
        assert_eq!(r.globs[0].supported_matched, 0);
    }
}
