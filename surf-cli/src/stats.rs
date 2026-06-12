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
    // A bad hub glob in surf.toml must fail loudly: silently dropping it excludes hubs from the
    // metrics with no signal (#38). stats already fails loudly when git can't answer; do the same
    // for malformed config rather than reporting on a quietly-narrowed hub set.
    let patterns: Vec<glob::Pattern> = ws
        .config
        .hubs
        .iter()
        .map(|p| {
            glob::Pattern::new(p).map_err(|e| anyhow!("invalid hub glob \"{p}\" in surf.toml: {e}"))
        })
        .collect::<Result<Vec<_>>>()?;

    // Unlike check's advisory git, stats *is* a history report: if git can't answer, fail loudly
    // rather than printing a misleading zero.
    let stream = git::log_stream(&ws.root, since, until)
        .ok_or_else(|| anyhow!("git history unavailable (not a repo, or a shallow clone?)"))?;

    // The whole window arrives in one spawn (#72); claim state is then propagated incrementally.
    // A commit's claim set derives solely from its hub files, so it *is* its first parent's
    // unless the commit touched a hub path — then only those hubs are re-read (`git show`), which
    // costs ~hub-edits, not ~commits. Merges are in the stream purely as state carriers: their
    // first-parent diff includes everything the other parents brought in, so applying it to the
    // first parent's state reproduces the merge's tree exactly.
    let is_hub = |path: &str| patterns.iter().any(|p| p.matches(path));
    let mut state: HashMap<&str, Rc<HubState>> = HashMap::new();
    // A first parent outside the stream (the edge of a --since window) is reconstructed once the
    // slow way — full ls-tree + per-hub show — and memoized. Root commits have no parent at all.
    let mut boundary: HashMap<String, Rc<HubState>> = HashMap::new();
    let empty: Rc<HubState> = Rc::new(HashMap::new());

    // --topo-order lists children before parents, so walking the stream in reverse means every
    // in-window first parent is resolved before its children ask for it.
    for rec in stream.iter().rev() {
        let parent_state =
            match rec.parents.first() {
                None => Rc::clone(&empty),
                Some(p) => match state.get(p.as_str()) {
                    Some(s) => Rc::clone(s),
                    None => Rc::clone(boundary.entry(p.clone()).or_insert_with(|| {
                        Rc::new(group_by_hub(claims_at(&ws.root, p, &patterns)))
                    })),
                },
            };

        let hub_changes: Vec<&(char, String)> = rec
            .changes
            .iter()
            .filter(|(_, path)| is_hub(path))
            .collect();
        let next = if hub_changes.is_empty() {
            parent_state
        } else {
            let mut next: HubState = (*parent_state).clone();
            for (status, path) in hub_changes {
                // Unreadable or unparsable hubs drop out of the set, exactly as claims_at skips
                // them when listing a full tree.
                let claims = (*status != 'D')
                    .then(|| hub_claims(&ws.root, &rec.sha, path))
                    .flatten();
                match claims {
                    Some(claims) => next.insert(path.clone(), Rc::new(claims)),
                    None => next.remove(path.as_str()),
                };
            }
            Rc::new(next)
        };
        state.insert(&rec.sha, next);
    }

    let (mut rs_n, mut rs_d, mut ip_n, mut ip_d) = (0, 0, 0, 0);
    let mut commits = 0;

    for rec in &stream {
        // Merges are excluded from the metrics themselves: one non-merge commit = one PR.
        if rec.parents.len() > 1 {
            continue;
        }
        commits += 1;
        // Root commits carry no `before` to compare against (the old per-commit diff-tree showed
        // nothing for them), and empty commits have no events by definition.
        let Some(parent) = rec.parents.first() else {
            continue;
        };
        if rec.changes.is_empty() {
            continue;
        }
        let changed: HashSet<&str> = rec.changes.iter().map(|(_, p)| p.as_str()).collect();

        let after = &state[rec.sha.as_str()];
        // The state pass resolved every first parent into `state` or `boundary` already.
        let before = state
            .get(parent.as_str())
            .or_else(|| boundary.get(parent.as_str()))
            .unwrap_or(&empty);

        for (hub, claims) in after.iter() {
            let before_claims = before.get(hub);
            for c in claims.iter() {
                let prev = before_claims.and_then(|cs| cs.iter().find(|p| p.id == c.id));

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
                if c.files.iter().any(|f| changed.contains(f.as_str())) {
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
    }

    Ok(StatsReport {
        version: REPORT_VERSION,
        since: since.map(str::to_string),
        until: until.map(str::to_string),
        commits,
        rubber_stamp: Rate::new(rs_n, rs_d),
        in_place: Rate::new(ip_n, ip_d),
    })
}

/// Claim sets keyed by hub path — the unit the incremental walk replaces when a commit touches
/// a hub. `Rc` lets unchanged hubs (the overwhelming majority) be shared across commit states.
type HubState = HashMap<String, Rc<Vec<ClaimRec>>>;

fn group_by_hub(claims: Vec<ClaimRec>) -> HubState {
    let mut out: HashMap<String, Vec<ClaimRec>> = HashMap::new();
    for c in claims {
        out.entry(c.hub.clone()).or_default().push(c);
    }
    out.into_iter().map(|(k, v)| (k, Rc::new(v))).collect()
}

/// The claims of a single hub as it exists at `rev`, or `None` if it can't be read or parsed.
fn hub_claims(root: &std::path::Path, rev: &str, hub: &str) -> Option<Vec<ClaimRec>> {
    let content = git::show(root, rev, hub)?;
    let parsed = parse_hub(&content).ok()?;
    let mut out = Vec::new();
    for claim in &parsed.frontmatter.anchors {
        let sites = claim.at.sites();
        let files = sites
            .iter()
            .filter_map(|s| parse_anchor(s).ok().map(|a| a.file))
            .collect();
        out.push(ClaimRec {
            hub: hub.to_string(),
            id: sites.join(" + "),
            hash: claim.hash.clone(),
            prose: claim.claim.trim().to_string(),
            files,
        });
    }
    Some(out)
}

/// Every claim across the hub set as it existed at `rev`, or empty if the rev/hubs don't exist.
/// The slow path — one full `ls-tree` plus a `show` per hub — kept only for first parents that
/// fall outside the streamed window (#72).
fn claims_at(root: &std::path::Path, rev: &str, patterns: &[glob::Pattern]) -> Vec<ClaimRec> {
    let mut out = Vec::new();
    let files = git::list_files_at(root, rev).unwrap_or_default();
    for hub in files
        .iter()
        .filter(|f| patterns.iter().any(|p| p.matches(f)))
    {
        out.extend(hub_claims(root, rev, hub).unwrap_or_default());
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

    fn commit_at(root: &Path, msg: &str, date: &str) {
        git(root, &["add", "."]);
        let status = Command::new("git")
            .current_dir(root)
            .args(["-c", "user.email=t@t", "-c", "user.name=t"])
            .env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date)
            .args(["commit", "-q", "-m", msg])
            .status()
            .unwrap();
        assert!(status.success(), "git commit at {date} failed");
    }

    fn ws(root: &Path) -> Workspace {
        Workspace::discover(root).unwrap()
    }

    #[test]
    fn invalid_hub_glob_syntax_errors() {
        // A malformed hub glob in surf.toml must fail loudly, not silently exclude hubs (#38).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "surf.toml", "hubs = [\"hubs/[.md\"]\n");
        let err = compute(&ws(root), None, None).unwrap_err();
        assert!(err.to_string().contains("hubs/["), "got: {err}");
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
        write(
            root,
            "hubs/a.md",
            &hub("add increments", anchor, &rust_hash(v1, anchor)),
        );
        commit(root, "seed");

        // Each later commit edits the body and re-seals with identical prose — a rubber stamp.
        for n in 2..=4 {
            let v = format!("pub fn add(a: i64) -> i64 {{ a + {n} }}\n");
            write(root, "src/m.rs", &v);
            write(
                root,
                "hubs/a.md",
                &hub("add increments", anchor, &rust_hash(&v, anchor)),
            );
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
    fn merge_commits_carry_state_but_do_not_count() {
        // A re-stamp lands on a feature branch; main moves independently; the branch is merged
        // with --no-ff. The merge must propagate the branch's hub state (its first-parent diff
        // carries it) without itself counting as a commit or an event, and a post-merge re-stamp
        // must see the merged state as its `before`.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        git(root, &["init", "-q", "-b", "main"]);
        write(root, "surf.toml", "");

        let anchor = "src/m.rs > add";
        let v1 = "pub fn add(a: i64) -> i64 { a + 1 }\n";
        write(root, "src/m.rs", v1);
        write(
            root,
            "hubs/a.md",
            &hub("add increments", anchor, &rust_hash(v1, anchor)),
        );
        commit(root, "seed");

        git(root, &["checkout", "-q", "-b", "feature"]);
        let v2 = "pub fn add(a: i64) -> i64 { a + 2 }\n";
        write(root, "src/m.rs", v2);
        write(
            root,
            "hubs/a.md",
            &hub("add increments", anchor, &rust_hash(v2, anchor)),
        );
        commit(root, "branch re-stamp");

        git(root, &["checkout", "-q", "main"]);
        write(root, "other.txt", "unrelated\n");
        commit(root, "unrelated main work");

        git(
            root,
            &["merge", "-q", "--no-ff", "-m", "merge feature", "feature"],
        );

        let v3 = "pub fn add(a: i64) -> i64 { a + 3 }\n";
        write(root, "src/m.rs", v3);
        write(
            root,
            "hubs/a.md",
            &hub("add increments", anchor, &rust_hash(v3, anchor)),
        );
        commit(root, "post-merge re-stamp");

        let r = compute(&ws(root), None, None).unwrap();
        // seed + branch + unrelated + post-merge; the merge itself is excluded.
        assert_eq!(r.commits, 4);
        // Both re-stamps count — the post-merge one only if the merge carried the v2 state.
        assert_eq!((r.rubber_stamp.n, r.rubber_stamp.d), (2, 2));
        assert_eq!((r.in_place.n, r.in_place.d), (2, 2));
    }

    #[test]
    fn since_window_reconstructs_the_out_of_window_parent() {
        // The seed predates the --since window, so the in-window re-stamp's first parent is not
        // in the stream. Its state must be reconstructed (the slow ls-tree fallback) rather than
        // treated as empty — otherwise the re-stamp would read as a brand-new claim and the
        // rubber-stamp event would be lost.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        git(root, &["init", "-q", "-b", "main"]);
        write(root, "surf.toml", "");

        let anchor = "src/m.rs > add";
        let v1 = "pub fn add(a: i64) -> i64 { a + 1 }\n";
        write(root, "src/m.rs", v1);
        write(
            root,
            "hubs/a.md",
            &hub("add increments", anchor, &rust_hash(v1, anchor)),
        );
        commit_at(root, "seed", "2020-01-01T12:00:00 +0000");

        let v2 = "pub fn add(a: i64) -> i64 { a + 2 }\n";
        write(root, "src/m.rs", v2);
        write(
            root,
            "hubs/a.md",
            &hub("add increments", anchor, &rust_hash(v2, anchor)),
        );
        commit_at(root, "re-stamp", "2026-01-01T12:00:00 +0000");

        let r = compute(&ws(root), Some("2023-01-01"), None).unwrap();
        assert_eq!(r.commits, 1);
        assert_eq!((r.rubber_stamp.n, r.rubber_stamp.d), (1, 1));
        assert_eq!((r.in_place.n, r.in_place.d), (1, 1));
    }

    #[test]
    fn deleted_hub_drops_its_claims_from_later_state() {
        // After the hub is deleted, touching the formerly-anchored file must produce no events —
        // a stale incremental state that kept the dead hub's claims would inflate the in-place
        // denominator.
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

        fs::remove_file(root.join("hubs/a.md")).unwrap();
        commit(root, "drop hub");

        write(root, "src/m.rs", "pub fn add(a: i64) -> i64 { a + 9 }\n");
        commit(root, "bump unanchored");

        let r = compute(&ws(root), None, None).unwrap();
        assert_eq!(r.commits, 3);
        assert_eq!((r.rubber_stamp.d, r.in_place.d), (0, 0));
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
