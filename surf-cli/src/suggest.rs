//! `surf suggest <globs>` — propose anchors for public functions no hub covers yet (§8, #18).
//! Scans the given source files, lists each top-level public function that isn't already
//! anchored, and prints a copy-pasteable starter hub. Suggestions only: it never writes a file
//! and never stamps a hash — the author edits the claims and runs `surf verify`.

use crate::format::Format;
use crate::workspace::Workspace;
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashSet;
use std::process::ExitCode;
use surf_core::{parse_anchor, parse_hub, public_fns, Lang};

#[derive(Debug, Clone, Serialize)]
struct Suggestion {
    file: String,
    symbol: String,
    at: String,
}

pub fn run(ws: &Workspace, globs: &[String], format: Format) -> Result<ExitCode> {
    let covered = covered_symbols(ws)?;
    let suggestions = scan(ws, globs, &covered)?;

    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&suggestions)?),
        Format::Human => print_human(&suggestions),
    }
    Ok(ExitCode::SUCCESS)
}

/// `(file, first-segment)` pairs already anchored by some hub — the same coverage notion lint's
/// under-coverage check uses. A public fn matching one of these is already documented.
fn covered_symbols(ws: &Workspace) -> Result<HashSet<(String, String)>> {
    let mut covered = HashSet::new();
    for hub_path in ws.hub_paths()? {
        let content = std::fs::read_to_string(&hub_path)
            .with_context(|| format!("reading {}", hub_path.display()))?;
        let Ok(hub) = parse_hub(&content) else {
            continue;
        };
        for claim in &hub.frontmatter.anchors {
            for site in claim.at.sites() {
                if let Ok(anchor) = parse_anchor(site) {
                    if let Some(seg) = anchor.segments.first() {
                        covered.insert((anchor.file, seg.name.clone()));
                    }
                }
            }
        }
    }
    Ok(covered)
}

fn scan(
    ws: &Workspace,
    globs: &[String],
    covered: &HashSet<(String, String)>,
) -> Result<Vec<Suggestion>> {
    let mut out = Vec::new();
    for pattern in globs {
        let joined = ws.root.join(pattern);
        let pattern = joined
            .to_str()
            .with_context(|| format!("glob is not valid UTF-8: {}", joined.display()))?;
        for entry in glob::glob(pattern).context("invalid glob pattern")? {
            let path = entry?;
            if !path.is_file() {
                continue;
            }
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
            for symbol in public_fns(&source, lang) {
                if covered.contains(&(rel.clone(), symbol.clone())) {
                    continue;
                }
                let at = format!("{rel} > {symbol}");
                out.push(Suggestion {
                    file: rel.clone(),
                    symbol,
                    at,
                });
            }
        }
    }
    out.sort_by(|a, b| (&a.file, &a.symbol).cmp(&(&b.file, &b.symbol)));
    out.dedup_by(|a, b| a.at == b.at);
    Ok(out)
}

fn print_human(suggestions: &[Suggestion]) {
    if suggestions.is_empty() {
        println!("surf suggest: no unanchored public functions found.");
        return;
    }
    println!(
        "# {} unanchored public function(s). Paste into a hub (or `surf new <name>`), write the",
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
        let covered = covered_symbols(&ws).unwrap();
        let s = scan(&ws, &["src/*.rs".to_string()], &covered).unwrap();
        // `a` is anchored, `c` is private — only `b` remains.
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].at, "src/m.rs > b");
        assert_eq!(s[0].symbol, "b");
    }

    #[test]
    fn unsupported_files_are_skipped() {
        let (_t, ws) = ws_with(&[("notes.txt", "pub fn a() {}\n")]);
        let covered = covered_symbols(&ws).unwrap();
        let s = scan(&ws, &["*.txt".to_string()], &covered).unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn json_shape_has_no_hash() {
        let (_t, ws) = ws_with(&[("src/m.rs", "pub fn solo() {}\n")]);
        let covered = covered_symbols(&ws).unwrap();
        let s = scan(&ws, &["src/*.rs".to_string()], &covered).unwrap();
        let json = serde_json::to_value(&s).unwrap();
        let obj = json[0].as_object().unwrap();
        for key in ["file", "symbol", "at"] {
            assert!(obj.contains_key(key), "missing `{key}` in {obj:?}");
        }
        assert!(!obj.contains_key("hash"));
    }
}
