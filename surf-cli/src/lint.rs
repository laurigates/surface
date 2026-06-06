//! `surf lint` (§9.1.2): every anchor must resolve to exactly one symbol. Ambiguous or
//! vanished anchors block; a symbol that was merely renamed (detected via stored-hash
//! match, §6.4) only warns and points at `surf verify --follow`.

use crate::workspace::Workspace;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::ExitCode;
use surf_core::{find_renamed, parse_anchor, parse_hub, resolve, Lang, ResolveError};

#[derive(Debug, PartialEq, Eq)]
pub enum Severity {
    Block,
    Warn,
}

#[derive(Debug)]
pub struct Finding {
    pub severity: Severity,
    pub hub: String,
    pub claim: String,
    pub at: String,
    pub message: String,
}

pub fn run(ws: &Workspace) -> Result<ExitCode> {
    let findings = lint_workspace(ws)?;
    let blocks = findings
        .iter()
        .filter(|f| f.severity == Severity::Block)
        .count();
    let warns = findings.len() - blocks;

    for f in &findings {
        let tag = match f.severity {
            Severity::Block => "error",
            Severity::Warn => "warning",
        };
        println!("{tag}: {} :: {}", f.hub, f.at);
        println!("    {}", f.message);
        println!("    claim: {}", truncate(&f.claim, 80));
    }

    if findings.is_empty() {
        println!("surf lint: all anchors resolve.");
    } else {
        println!("surf lint: {blocks} error(s), {warns} warning(s).");
    }

    Ok(if blocks > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn lint_workspace(ws: &Workspace) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for hub_path in ws.hub_paths()? {
        let rel = hub_path
            .strip_prefix(&ws.root)
            .unwrap_or(&hub_path)
            .display()
            .to_string();
        let content = std::fs::read_to_string(&hub_path)
            .with_context(|| format!("reading {}", hub_path.display()))?;

        let hub = match parse_hub(&content) {
            Ok(hub) => hub,
            Err(e) => {
                findings.push(Finding {
                    severity: Severity::Block,
                    hub: rel,
                    claim: String::new(),
                    at: String::new(),
                    message: format!("invalid hub: {e}"),
                });
                continue;
            }
        };

        for claim in &hub.frontmatter.anchors {
            for site in claim.at.sites() {
                lint_site(
                    ws,
                    &rel,
                    &claim.claim,
                    site,
                    claim.hash.as_deref(),
                    &mut findings,
                );
            }
        }
    }
    Ok(findings)
}

fn lint_site(
    ws: &Workspace,
    hub: &str,
    claim: &str,
    site: &str,
    stored_hash: Option<&str>,
    findings: &mut Vec<Finding>,
) {
    let mut block = |message: String| {
        findings.push(Finding {
            severity: Severity::Block,
            hub: hub.to_string(),
            claim: claim.to_string(),
            at: site.to_string(),
            message,
        });
    };

    let anchor = match parse_anchor(site) {
        Ok(a) => a,
        Err(e) => return block(format!("invalid anchor: {e}")),
    };
    let Some(lang) = Lang::from_path(&anchor.file) else {
        return block(format!("unsupported file type: {}", anchor.file));
    };
    let path: PathBuf = ws.root.join(&anchor.file);
    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            return block(format!(
                "cannot read `{}` (file moved or removed?)",
                anchor.file
            ))
        }
    };

    match resolve(&source, lang, &anchor) {
        Ok(_) => {}
        Err(ResolveError::Ambiguous { segment, count }) => {
            block(format!("`{segment}` is ambiguous ({count} matches); disambiguate with `@N`"));
        }
        Err(ResolveError::Parse) => block(format!("could not parse `{}`", anchor.file)),
        Err(ResolveError::NotFound { segment }) => match stored_hash {
            Some(h) => match find_renamed(&source, lang, h) {
                Ok(Some(new_name)) => findings.push(Finding {
                    severity: Severity::Warn,
                    hub: hub.to_string(),
                    claim: claim.to_string(),
                    at: site.to_string(),
                    message: format!(
                        "`{segment}` not found, but its code appears to live under `{new_name}` now — run `surf verify --follow`"
                    ),
                }),
                Ok(None) => block(format!("`{segment}` not found and no current symbol matches the stored hash — the claim points at nothing")),
                Err(e) => block(format!("`{segment}` not found; rename check failed: {e}")),
            },
            None => block(format!("`{segment}` not found (claim has no stored hash to match against)")),
        },
    }
}

fn truncate(s: &str, max: usize) -> String {
    let one_line = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= max {
        one_line
    } else {
        let kept: String = one_line.chars().take(max).collect();
        format!("{kept}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use surf_core::hash_anchor;

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

    fn rust_hash(src: &str, anchor: &str) -> String {
        hash_anchor(src, Lang::Rust, &parse_anchor(anchor).unwrap()).unwrap()
    }

    #[test]
    fn clean_anchor_has_no_findings() {
        let (_t, ws) = ws_with(&[
            ("src/auth.rs", "pub fn greet() -> &'static str { \"hi\" }\n"),
            ("hubs/a.md", "---\nsummary: x\nanchors:\n  - claim: greeting exists\n    at: src/auth.rs > greet\n---\n"),
        ]);
        assert!(lint_workspace(&ws).unwrap().is_empty());
    }

    #[test]
    fn ambiguous_anchor_blocks() {
        let (_t, ws) = ws_with(&[
            (
                "src/dup.ts",
                "function dup(): void {}\nfunction dup(): void {}\n",
            ),
            (
                "hubs/a.md",
                "---\nsummary: x\nanchors:\n  - claim: dup\n    at: src/dup.ts > dup\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Block);
        assert!(
            f[0].message.contains("@N"),
            "message should suggest @N: {}",
            f[0].message
        );
    }

    #[test]
    fn vanished_symbol_blocks() {
        let (_t, ws) = ws_with(&[
            ("src/auth.rs", "pub fn greet() {}\n"),
            (
                "hubs/a.md",
                "---\nsummary: x\nanchors:\n  - claim: ghost\n    at: src/auth.rs > ghost\n---\n",
            ),
        ]);
        let f = lint_workspace(&ws).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Block);
    }

    #[test]
    fn renamed_symbol_warns_and_suggests_follow() {
        let new_src = "pub fn rotate_token(t: &str) -> String { t.to_string() }\n";
        let stored = rust_hash(new_src, "src/auth.rs > rotate_token");
        let hub = format!(
            "---\nsummary: x\nanchors:\n  - claim: rotation\n    at: src/auth.rs > rotate\n    hash: {stored}\n---\n"
        );
        let (_t, ws) = ws_with(&[("src/auth.rs", new_src), ("hubs/a.md", &hub)]);

        let f = lint_workspace(&ws).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warn);
        assert!(f[0].message.contains("rotate_token"));
        assert!(f[0].message.contains("--follow"));
    }
}
