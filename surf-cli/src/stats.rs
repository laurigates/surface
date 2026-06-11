//! `surf stats` — adoption + rubber-stamp metrics from git history (§9.2, #13).
//!
//! Two falsifiable adopt/kill signals, computed by walking commits in a date range:
//!
//! - **rubber-stamp rate** — of re-stamp events (a claim's stored `hash:` changed value in a
//!   commit), the share where the claim's *prose* was left untouched. A high rate means people
//!   re-seal to clear the gate without re-reading the claim — the gate is being routed around.
//! - **in-place update rate** — of claim-touch events (a commit changed a file a claim anchors),
//!   the share where the claim's stored hash was updated in the *same* commit. A high rate means
//!   docs travel with the code rather than drifting.
//!
//! Heuristics, deliberately so (the metrics are advisory, not a gate):
//! - One commit = one PR (merges excluded). Squash-merge workflows map cleanly; merge-commit
//!   workflows attribute the work to its individual commits.
//! - Claim identity is its `at:` site(s); renaming a `claim:`'s anchor reads as a new claim.
//! - The in-place denominator counts *any* change to an anchored file, including hash-neutral
//!   edits (comments, formatting) that wouldn't actually diverge the claim — so the true rate is
//!   at least the reported one. Surfaced rather than hidden.

use crate::format::Format;
use crate::git;
use crate::workspace::Workspace;
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::process::ExitCode;
use std::rc::Rc;
use surf_core::{parse_anchor, parse_hub, REPORT_VERSION};

#[derive(Debug, Clone)]
struct ClaimRec {
    hub: String,
    id: String,
    hash: Option<String>,
    prose: String,
    files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Rate {
    n: usize,
    d: usize,
    rate: Option<f64>,
}

impl Rate {
    fn new(n: usize, d: usize) -> Rate {
        let rate = (d > 0).then(|| n as f64 / d as f64);
        Rate { n, d, rate }
    }
}

#[derive(Debug, Clone, Serialize)]
struct StatsReport {
    version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    until: Option<String>,
    commits: usize,
    rubber_stamp: Rate,
    in_place: Rate,
}

pub fn run(
    ws: &Workspace,
    since: Option<&str>,
    until: Option<&str>,
    format: Format,
) -> Result<ExitCode> {
    let report = compute(ws, since, until)?;

    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        Format::Human => print_human(&report),
    }
    Ok(ExitCode::SUCCESS)
}

fn compute(ws: &Workspace, since: Option<&str>, until: Option<&str>) -> Result<StatsReport> {
    let patterns: Vec<glob::Pattern> = ws
        .config
        .hubs
        .iter()
        .filter_map(|p| glob::Pattern::new(p).ok())
        .collect();

    // Unlike check's advisory git, stats *is* a history report: if git can't answer, fail loudly
    // rather than printing a misleading zero.
    let commits = git::log_commits(&ws.root, since, until)
        .ok_or_else(|| anyhow!("git history unavailable (not a repo, or a shallow clone?)"))?;

    let (mut rs_n, mut rs_d, mut ip_n, mut ip_d) = (0, 0, 0, 0);

    // Each commit needs the claim set at the commit (`after`) and at its parent (`before`); each
    // `claims_at` spawns `git ls-tree` plus a `git show` per hub. Memoize by full commit SHA so a
    // rev is walked at most once. On linear history `before(commit)` is `after(parent)`, so once
    // the parent's SHA is canonicalized the two share a key and ~half the work is a cache hit;
    // merges/gaps simply miss and populate the cache. `Rc` shares the cached set without a deep
    // clone per hit.
    let mut cache: HashMap<String, Rc<Vec<ClaimRec>>> = HashMap::new();
    let mut claims_for = |sha: &str| -> Rc<Vec<ClaimRec>> {
        if let Some(hit) = cache.get(sha) {
            return Rc::clone(hit);
        }
        let computed = Rc::new(claims_at(&ws.root, sha, &patterns));
        cache.insert(sha.to_string(), Rc::clone(&computed));
        computed
    };

    for sha in &commits {
        let changed: HashSet<String> = git::commit_files(&ws.root, sha)
            .unwrap_or_default()
            .into_iter()
            .collect();
        if changed.is_empty() {
            continue;
        }

        let after = claims_for(sha);
        // Canonicalize `sha^` to the parent's SHA so `before` reuses the parent's cached `after`.
        let before = match git::rev_parse(&ws.root, &format!("{sha}^")) {
            Some(parent) => claims_for(&parent),
            None => Rc::new(Vec::new()),
        };
        let before_by_key: HashMap<(&str, &str), &ClaimRec> = before
            .iter()
            .map(|c| ((c.hub.as_str(), c.id.as_str()), c))
            .collect();

        for c in after.iter() {
            let prev = before_by_key.get(&(c.hub.as_str(), c.id.as_str())).copied();

            // Rubber-stamp: an already-sealed claim whose hash value changed this commit.
            if let (Some(prev), Some(new_hash)) = (prev, &c.hash) {
                if let Some(old_hash) = &prev.hash {
                    if old_hash != new_hash {
                        rs_d += 1;
                        if prev.prose == c.prose {
                            rs_n += 1;
                        }
                    }
                }
            }

            // In-place: a commit that touched an anchored file (seeded domain).
            if c.files.iter().any(|f| changed.contains(f)) {
                ip_d += 1;
                let updated = match prev {
                    Some(prev) => prev.hash != c.hash,
                    None => c.hash.is_some(),
                };
                if updated {
                    ip_n += 1;
                }
            }
        }
    }

    Ok(StatsReport {
        version: REPORT_VERSION,
        since: since.map(str::to_string),
        until: until.map(str::to_string),
        commits: commits.len(),
        rubber_stamp: Rate::new(rs_n, rs_d),
        in_place: Rate::new(ip_n, ip_d),
    })
}

/// Every claim across the hub set as it existed at `rev`, or empty if the rev/hubs don't exist.
fn claims_at(root: &std::path::Path, rev: &str, patterns: &[glob::Pattern]) -> Vec<ClaimRec> {
    let mut out = Vec::new();
    let files = git::list_files_at(root, rev).unwrap_or_default();
    for hub in files
        .iter()
        .filter(|f| patterns.iter().any(|p| p.matches(f)))
    {
        let Some(content) = git::show(root, rev, hub) else {
            continue;
        };
        let Ok(parsed) = parse_hub(&content) else {
            continue;
        };
        for claim in &parsed.frontmatter.anchors {
            let sites = claim.at.sites();
            let files = sites
                .iter()
                .filter_map(|s| parse_anchor(s).ok().map(|a| a.file))
                .collect();
            out.push(ClaimRec {
                hub: hub.clone(),
                id: sites.join(" + "),
                hash: claim.hash.clone(),
                prose: claim.claim.trim().to_string(),
                files,
            });
        }
    }
    out
}

fn pct(rate: &Rate) -> String {
    match rate.rate {
        Some(r) => format!("{:.0}% ({}/{})", r * 100.0, rate.n, rate.d),
        None => "n/a (no events)".to_string(),
    }
}

fn print_human(report: &StatsReport) {
    let range = match (&report.since, &report.until) {
        (Some(s), Some(u)) => format!("{s}..{u}"),
        (Some(s), None) => format!("since {s}"),
        (None, Some(u)) => format!("until {u}"),
        (None, None) => "all history".to_string(),
    };
    println!("surf stats ({range}, {} commits)", report.commits);
    println!("  rubber-stamp rate:    {}", pct(&report.rubber_stamp));
    println!("    re-stamps that left the claim's prose untouched");
    println!("  in-place update rate: {}", pct(&report.in_place));
    println!("    anchored-file touches that re-sealed the claim in the same commit");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use surf_core::{hash_anchor, parse_anchor, Lang};

    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(root)
            .args(["-c", "user.email=t@t", "-c", "user.name=t"])
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, content).unwrap();
    }

    fn rust_hash(src: &str, anchor: &str) -> String {
        hash_anchor(src, Lang::Rust, &parse_anchor(anchor).unwrap()).unwrap()
    }

    fn hub(claim: &str, at: &str, hash: &str) -> String {
        format!(
            "---\nsummary: x\nanchors:\n  - claim: {claim}\n    at: {at}\n    hash: {hash}\n---\n"
        )
    }

    fn commit(root: &Path, msg: &str) {
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", msg]);
    }

    fn ws(root: &Path) -> Workspace {
        Workspace::discover(root).unwrap()
    }

    #[test]
    fn rubber_stamp_when_hash_changes_but_prose_does_not() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        git(root, &["init", "-q", "-b", "main"]);
        write(root, "surf.toml", "");

        let v1 = "pub fn add(a: i64) -> i64 { a + 1 }\n";
        write(root, "src/m.rs", v1);
        write(
            root,
            "hubs/a.md",
            &hub(
                "add increments",
                "src/m.rs > add",
                &rust_hash(v1, "src/m.rs > add"),
            ),
        );
        commit(root, "seed");

        // Code changes; the claim is re-sealed with the SAME prose — a rubber stamp.
        let v2 = "pub fn add(a: i64) -> i64 { a + 2 }\n";
        write(root, "src/m.rs", v2);
        write(
            root,
            "hubs/a.md",
            &hub(
                "add increments",
                "src/m.rs > add",
                &rust_hash(v2, "src/m.rs > add"),
            ),
        );
        commit(root, "bump and re-stamp");

        let r = compute(&ws(root), None, None).unwrap();
        assert_eq!((r.rubber_stamp.n, r.rubber_stamp.d), (1, 1));
        // The same commit touched the anchored file and updated the hash → in-place.
        assert_eq!((r.in_place.n, r.in_place.d), (1, 1));
    }

    #[test]
    fn memoized_before_state_is_correct_across_a_linear_chain() {
        // Three re-stamps over a 4-commit linear history. Each commit's `before` is its parent's
        // `after`, served from the SHA-keyed cache — a regression in that reuse (stale or
        // mis-keyed entry) would skew the counts away from the expected 3/3.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        git(root, &["init", "-q", "-b", "main"]);
        write(root, "surf.toml", "");

        let anchor = "src/m.rs > add";
        let v1 = "pub fn add(a: i64) -> i64 { a + 1 }\n";
        write(root, "src/m.rs", v1);
        write(root, "hubs/a.md", &hub("add increments", anchor, &rust_hash(v1, anchor)));
        commit(root, "seed");

        // Each later commit edits the body and re-seals with identical prose — a rubber stamp.
        for n in 2..=4 {
            let v = format!("pub fn add(a: i64) -> i64 {{ a + {n} }}\n");
            write(root, "src/m.rs", &v);
            write(root, "hubs/a.md", &hub("add increments", anchor, &rust_hash(&v, anchor)));
            commit(root, &format!("bump {n}"));
        }

        let r = compute(&ws(root), None, None).unwrap();
        // Three re-stamp commits, all prose-unchanged; each also touched the anchored file.
        assert_eq!((r.rubber_stamp.n, r.rubber_stamp.d), (3, 3));
        assert_eq!((r.in_place.n, r.in_place.d), (3, 3));
    }

    #[test]
    fn genuine_update_is_not_a_rubber_stamp() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        git(root, &["init", "-q", "-b", "main"]);
        write(root, "surf.toml", "");

        let v1 = "pub fn add(a: i64) -> i64 { a + 1 }\n";
        write(root, "src/m.rs", v1);
        write(
            root,
            "hubs/a.md",
            &hub(
                "add adds one",
                "src/m.rs > add",
                &rust_hash(v1, "src/m.rs > add"),
            ),
        );
        commit(root, "seed");

        let v2 = "pub fn add(a: i64) -> i64 { a + 2 }\n";
        write(root, "src/m.rs", v2);
        write(
            root,
            "hubs/a.md",
            &hub(
                "add adds two",
                "src/m.rs > add",
                &rust_hash(v2, "src/m.rs > add"),
            ),
        );
        commit(root, "bump and re-document");

        let r = compute(&ws(root), None, None).unwrap();
        assert_eq!((r.rubber_stamp.n, r.rubber_stamp.d), (0, 1));
    }

    #[test]
    fn touching_anchored_code_without_resealing_is_not_in_place() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        git(root, &["init", "-q", "-b", "main"]);
        write(root, "surf.toml", "");

        let v1 = "pub fn add(a: i64) -> i64 { a + 1 }\n";
        write(root, "src/m.rs", v1);
        write(
            root,
            "hubs/a.md",
            &hub(
                "add increments",
                "src/m.rs > add",
                &rust_hash(v1, "src/m.rs > add"),
            ),
        );
        commit(root, "seed");

        // Change the anchored file but DON'T touch the hub — drift left in place.
        write(root, "src/m.rs", "pub fn add(a: i64) -> i64 { a + 9 }\n");
        commit(root, "bump only");

        let r = compute(&ws(root), None, None).unwrap();
        assert_eq!((r.in_place.n, r.in_place.d), (0, 1));
        // No hash value change happened, so no rubber-stamp event either.
        assert_eq!(r.rubber_stamp.d, 0);
    }

    #[test]
    fn errors_when_not_a_git_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "surf.toml", "");
        assert!(compute(&ws(root), None, None).is_err());
    }

    #[test]
    fn json_envelope_is_versioned() {
        let report = StatsReport {
            version: REPORT_VERSION,
            since: Some("2026-01-01".to_string()),
            until: None,
            commits: 3,
            rubber_stamp: Rate::new(1, 2),
            in_place: Rate::new(0, 0),
        };
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["version"], REPORT_VERSION);
        assert_eq!(json["rubber_stamp"]["rate"], 0.5);
        assert!(json["in_place"]["rate"].is_null());
        assert!(json.get("until").is_none());
    }
}
