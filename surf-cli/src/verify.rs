//! `surf verify` — the human escape hatch (§8, §9.1.4). Re-hash anchors after a human has
//! confirmed the prose still holds and write the hash back into the frontmatter ("I looked,
//! still true"). `--follow` re-points a renamed single-segment anchor and re-hashes in one
//! step (§6.4). Writes are surgical (only the touched line changes) and skipped entirely
//! when nothing changed, so a no-op verify leaves the file byte-identical.

use crate::workspace::Workspace;
use anyhow::{Context, Result};
use std::process::ExitCode;
use surf_core::{
    combine_site_hashes, find_renamed, hash_anchor, parse_anchor, parse_hub, set_anchor_at,
    set_anchor_hash, Lang,
};

enum Plan {
    Hash(String),
    Follow { new_at: String, new_hash: String },
    Skip(String),
}

pub fn run(ws: &Workspace, target: Option<&str>, follow: bool) -> Result<ExitCode> {
    let mut stamped = 0usize;
    let mut skipped: Vec<String> = Vec::new();
    let mut matched_any = false;

    for hub_path in ws.hub_paths()? {
        let rel = hub_path
            .strip_prefix(&ws.root)
            .unwrap_or(&hub_path)
            .display()
            .to_string();
        let original = std::fs::read_to_string(&hub_path)
            .with_context(|| format!("reading {}", hub_path.display()))?;
        let Ok(hub) = parse_hub(&original) else {
            continue;
        };
        let mut text = original.clone();

        for (idx, claim) in hub.frontmatter.anchors.iter().enumerate() {
            let sites = claim.at.sites();
            if let Some(t) = target {
                if !sites.iter().any(|s| s == t) {
                    continue;
                }
            }
            matched_any = true;
            let label = format!("{rel} :: {}", sites.join("  +  "));

            match plan_claim(ws, claim, follow) {
                Plan::Hash(new_hash) => match set_anchor_hash(&text, idx, &new_hash) {
                    Some(updated) => {
                        text = updated;
                        stamped += 1;
                    }
                    None => skipped.push(format!("{label} (could not write hash)")),
                },
                Plan::Follow { new_at, new_hash } => {
                    match set_anchor_at(&text, idx, &new_at)
                        .and_then(|t| set_anchor_hash(&t, idx, &new_hash))
                    {
                        Some(updated) => {
                            text = updated;
                            stamped += 1;
                            println!("followed {label} → {new_at}");
                        }
                        None => skipped.push(format!("{label} (could not rewrite at:)")),
                    }
                }
                Plan::Skip(reason) => skipped.push(format!("{label} ({reason})")),
            }
        }

        if text != original {
            std::fs::write(&hub_path, &text)
                .with_context(|| format!("writing {}", hub_path.display()))?;
            println!("updated {rel}");
        }
    }

    if let Some(t) = target {
        if !matched_any {
            anyhow::bail!("no anchor matching `{t}`");
        }
    }

    for s in &skipped {
        println!("skipped {s}");
    }
    println!(
        "surf verify: stamped {stamped} anchor(s), {} skipped.",
        skipped.len()
    );

    Ok(if skipped.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn plan_claim(ws: &Workspace, claim: &surf_core::Claim, follow: bool) -> Plan {
    let sites = claim.at.sites();

    let mut site_hashes = Vec::with_capacity(sites.len());
    let mut all_resolved = true;
    for site in sites {
        match hash_site(ws, site) {
            Some(h) => site_hashes.push(h),
            None => {
                all_resolved = false;
                break;
            }
        }
    }
    if all_resolved {
        return Plan::Hash(combine_site_hashes(&site_hashes));
    }

    if !follow {
        return Plan::Skip("does not resolve; run `surf lint`".into());
    }
    plan_follow(ws, claim)
}

fn plan_follow(ws: &Workspace, claim: &surf_core::Claim) -> Plan {
    let sites = claim.at.sites();
    if sites.len() != 1 {
        return Plan::Skip("--follow supports single-site anchors only".into());
    }
    let Some(stored) = claim.hash.as_deref() else {
        return Plan::Skip("--follow needs a stored hash to match against".into());
    };
    let Ok(anchor) = parse_anchor(&sites[0]) else {
        return Plan::Skip("invalid anchor".into());
    };
    if anchor.segments.len() != 1 {
        return Plan::Skip("--follow supports single-segment anchors only".into());
    }
    let Some(lang) = Lang::from_path(&anchor.file) else {
        return Plan::Skip("unsupported file type".into());
    };
    let Ok(source) = std::fs::read_to_string(ws.root.join(&anchor.file)) else {
        return Plan::Skip("cannot read source file".into());
    };

    match find_renamed(&source, lang, stored) {
        Ok(Some(new_name)) => {
            let new_at = format!("{} > {new_name}", anchor.file);
            match parse_anchor(&new_at)
                .ok()
                .and_then(|a| hash_anchor(&source, lang, &a).ok())
            {
                Some(new_hash) => Plan::Follow { new_at, new_hash },
                None => Plan::Skip("rename target did not re-resolve".into()),
            }
        }
        _ => Plan::Skip("does not resolve and no rename match; run `surf lint`".into()),
    }
}

fn hash_site(ws: &Workspace, site: &str) -> Option<String> {
    let anchor = parse_anchor(site).ok()?;
    let lang = Lang::from_path(&anchor.file)?;
    let source = std::fs::read_to_string(ws.root.join(&anchor.file)).ok()?;
    hash_anchor(&source, lang, &anchor).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, content).unwrap();
    }

    #[test]
    fn verify_stamps_then_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "surf.toml", "");
        write(
            root,
            "src/m.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a + b }\n",
        );
        write(
            root,
            "hubs/a.md",
            "---\nsummary: s\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n---\n# Body\n",
        );

        let ws = Workspace::discover(root).unwrap();
        run(&ws, None, false).unwrap();

        // Hash now present and equals the canonical hash of the symbol.
        let after = fs::read_to_string(root.join("hubs/a.md")).unwrap();
        let hub = parse_hub(&after).unwrap();
        let expected = hash_anchor(
            "pub fn add(a: i64, b: i64) -> i64 { a + b }\n",
            Lang::Rust,
            &parse_anchor("src/m.rs > add").unwrap(),
        )
        .unwrap();
        assert_eq!(
            hub.frontmatter.anchors[0].hash.as_deref(),
            Some(expected.as_str())
        );

        // Second verify is a no-op: byte-identical.
        run(&ws, None, false).unwrap();
        assert_eq!(fs::read_to_string(root.join("hubs/a.md")).unwrap(), after);
    }

    #[test]
    fn follow_repoints_renamed_anchor() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let src = "pub fn rotate_token(t: &str) -> String { t.to_string() }\n";
        let stored = hash_anchor(
            src,
            Lang::Rust,
            &parse_anchor("src/a.rs > rotate_token").unwrap(),
        )
        .unwrap();
        write(root, "surf.toml", "");
        write(root, "src/a.rs", src);
        write(
            root,
            "hubs/a.md",
            &format!("---\nsummary: s\nanchors:\n  - claim: rotation\n    at: src/a.rs > rotate\n    hash: {stored}\n---\n"),
        );

        let ws = Workspace::discover(root).unwrap();
        let code = run(&ws, None, true).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let hub = parse_hub(&fs::read_to_string(root.join("hubs/a.md")).unwrap()).unwrap();
        assert_eq!(
            hub.frontmatter.anchors[0].at.sites(),
            &["src/a.rs > rotate_token".to_string()]
        );
        assert_eq!(
            hub.frontmatter.anchors[0].hash.as_deref(),
            Some(stored.as_str())
        );
    }

    #[test]
    fn unresolved_without_follow_is_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "surf.toml", "");
        write(root, "src/m.rs", "pub fn other() {}\n");
        write(
            root,
            "hubs/a.md",
            "---\nsummary: s\nanchors:\n  - claim: c\n    at: src/m.rs > ghost\n---\n",
        );

        let ws = Workspace::discover(root).unwrap();
        let code = run(&ws, None, false).unwrap();
        assert_eq!(code, ExitCode::FAILURE);
        // Unchanged: no hash written.
        let hub = parse_hub(&fs::read_to_string(root.join("hubs/a.md")).unwrap()).unwrap();
        assert_eq!(hub.frontmatter.anchors[0].hash, None);
    }
}
