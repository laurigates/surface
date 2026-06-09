//! `surf for <path> [symbol]` — reverse lookup: which hubs/claims anchor into a file (#31).
//! The inverse of authoring — pull up the claims governing a file before you edit it. Reuses the
//! hub/anchor machinery and only matches on the anchored *path*, so it stays deterministic with
//! no model, network, or source parse. A query, not a gate: it always exits 0.

use crate::format::Format;
use crate::workspace::Workspace;
use anyhow::{Context, Result};
use serde::Serialize;
use std::process::ExitCode;
use surf_core::{parse_anchor, parse_hub, REPORT_VERSION};

#[derive(Debug, Clone, Serialize)]
struct ForMatch {
    hub: String,
    at: String,
    claim: String,
}

#[derive(Debug, Clone, Serialize)]
struct ForReport {
    version: u32,
    path: String,
    matches: Vec<ForMatch>,
}

pub fn run(ws: &Workspace, path: &str, symbol: Option<&str>, format: Format) -> Result<ExitCode> {
    let query = normalize(ws, path);
    let matches = find(ws, &query, symbol)?;

    match format {
        Format::Json => {
            let report = ForReport {
                version: REPORT_VERSION,
                path: query,
                matches,
            };
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Format::Human => print_human(&query, symbol, &matches),
    }
    Ok(ExitCode::SUCCESS)
}

/// To workspace-root-relative form (how anchors are written): strip a leading `./`, and make an
/// absolute path under the root relative. Anything else is taken as already root-relative.
fn normalize(ws: &Workspace, path: &str) -> String {
    let trimmed = path.strip_prefix("./").unwrap_or(path);
    let p = std::path::Path::new(trimmed);
    if p.is_absolute() {
        if let Ok(rel) = p.strip_prefix(&ws.root) {
            return rel.to_string_lossy().into_owned();
        }
    }
    trimmed.to_string()
}

fn find(ws: &Workspace, query: &str, symbol: Option<&str>) -> Result<Vec<ForMatch>> {
    let mut out = Vec::new();
    for hub_path in ws.hub_paths()? {
        let rel_hub = hub_path
            .strip_prefix(&ws.root)
            .unwrap_or(&hub_path)
            .display()
            .to_string();
        let content = std::fs::read_to_string(&hub_path)
            .with_context(|| format!("reading {}", hub_path.display()))?;
        let Ok(hub) = parse_hub(&content) else {
            // Malformed hubs are lint's job; skip rather than error out of a query.
            continue;
        };
        for claim in &hub.frontmatter.anchors {
            for site in claim.at.sites() {
                let Ok(anchor) = parse_anchor(site) else {
                    continue;
                };
                if anchor.file != query {
                    continue;
                }
                if let Some(sym) = symbol {
                    if anchor.segments.first().map(|s| s.name.as_str()) != Some(sym) {
                        continue;
                    }
                }
                out.push(ForMatch {
                    hub: rel_hub.clone(),
                    at: site.clone(),
                    claim: claim.claim.trim().to_string(),
                });
            }
        }
    }
    out.sort_by(|a, b| (&a.hub, &a.at).cmp(&(&b.hub, &b.at)));
    Ok(out)
}

fn print_human(query: &str, symbol: Option<&str>, matches: &[ForMatch]) {
    if matches.is_empty() {
        match symbol {
            Some(sym) => println!("surf for: no hubs anchor into {query} > {sym}."),
            None => println!("surf for: no hubs anchor into {query}."),
        }
        return;
    }
    let mut current_hub = "";
    for m in matches {
        if m.hub != current_hub {
            println!("{}", m.hub);
            current_hub = &m.hub;
        }
        println!("  {}", m.at);
        println!("    claim: {}", m.claim);
    }
    println!("surf for: {} claim(s) anchor into {query}.", matches.len());
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

    const HUB: &str = "---\nsummary: x\nanchors:\n  - claim: foo does a thing\n    at: src/x.rs > foo\n  - claim: bar does another\n    at: src/x.rs > bar\n  - claim: elsewhere\n    at: src/y.rs > baz\n---\n";

    #[test]
    fn finds_all_claims_anchoring_a_file() {
        let (_t, ws) = ws_with(&[("hubs/a.md", HUB)]);
        let m = find(&ws, "src/x.rs", None).unwrap();
        let ats: Vec<&str> = m.iter().map(|x| x.at.as_str()).collect();
        assert_eq!(ats, vec!["src/x.rs > bar", "src/x.rs > foo"]);
    }

    #[test]
    fn symbol_narrows_to_one() {
        let (_t, ws) = ws_with(&[("hubs/a.md", HUB)]);
        let m = find(&ws, "src/x.rs", Some("foo")).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].at, "src/x.rs > foo");
        assert_eq!(m[0].claim, "foo does a thing");
    }

    #[test]
    fn no_anchors_is_empty() {
        let (_t, ws) = ws_with(&[("hubs/a.md", HUB)]);
        assert!(find(&ws, "src/nope.rs", None).unwrap().is_empty());
    }

    #[test]
    fn normalize_strips_dot_slash_and_absolute_root() {
        let (_t, ws) = ws_with(&[("hubs/a.md", HUB)]);
        assert_eq!(normalize(&ws, "./src/x.rs"), "src/x.rs");
        let abs = ws.root.join("src/x.rs");
        assert_eq!(normalize(&ws, abs.to_str().unwrap()), "src/x.rs");
    }

    #[test]
    fn json_envelope_is_versioned() {
        let (_t, ws) = ws_with(&[("hubs/a.md", HUB)]);
        let matches = find(&ws, "src/x.rs", None).unwrap();
        let report = ForReport {
            version: REPORT_VERSION,
            path: "src/x.rs".to_string(),
            matches,
        };
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["version"], REPORT_VERSION);
        assert_eq!(json["path"], "src/x.rs");
        let first = json["matches"][0].as_object().unwrap();
        for key in ["hub", "at", "claim"] {
            assert!(first.contains_key(key), "missing `{key}`");
        }
    }
}
