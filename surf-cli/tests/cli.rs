//! Binary-level integration tests (#41). Unit tests already cover each command's logic and
//! JSON *shape* in-process; these invoke the built `surf` binary the way CI pipelines and
//! agents do — asserting the three things only the binary boundary can: process exit codes,
//! human stdout, and that `--format json` emits parseable JSON on stdout matching the
//! versioned contract (`REPORT_VERSION`, §5). serde_json + surf-core are reachable here as
//! normal dependencies of the crate under test.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::path::Path;
use surf_core::{hash_anchor, parse_anchor, Lang, REPORT_VERSION};

/// Write `content` to `root/rel`, creating parent dirs.
fn write(root: &Path, rel: &str, content: &str) {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(p, content).unwrap();
}

fn rust_hash(src: &str, anchor: &str) -> String {
    hash_anchor(src, Lang::Rust, &parse_anchor(anchor).unwrap()).unwrap()
}

/// `surf` rooted at `root` (so `surf.toml` discovery resolves there, not the test runner's cwd).
fn surf(root: &Path) -> Command {
    let mut cmd = Command::cargo_bin("surf").unwrap();
    cmd.current_dir(root);
    cmd
}

/// Minimal `git init` + one commit of the whole tree, so commands that read history
/// (`check --base`, `stats`) have something to walk. Identity is set per-invocation to keep
/// the test independent of the runner's global git config.
fn git(root: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .current_dir(root)
        .args(["-c", "user.email=t@t", "-c", "user.name=t"])
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

fn commit_all(root: &Path, msg: &str) {
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", msg]);
}

/// A workspace whose single Rust anchor is stamped clean (hash matches the on-disk source).
fn clean_workspace(root: &Path) {
    let src = "pub fn add(a: i64, b: i64) -> i64 { a + b }\n";
    write(root, "surf.toml", "");
    write(root, "src/m.rs", src);
    write(
        root,
        "hubs/a.md",
        &format!(
            "---\nsummary: x\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n    hash: {}\n---\n",
            rust_hash(src, "src/m.rs > add")
        ),
    );
}

/// Parse the command's stdout as JSON, asserting it is well-formed (the machine contract:
/// stdout must be pure JSON even when the command also emits warnings).
fn stdout_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout was not valid JSON: {e}\n--- stdout ---\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

// ---- exit codes -----------------------------------------------------------------------------

#[test]
fn check_clean_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    clean_workspace(tmp.path());
    surf(tmp.path())
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("all anchored spans match"));
}

#[test]
fn check_diverged_exits_one() {
    let tmp = tempfile::tempdir().unwrap();
    clean_workspace(tmp.path());
    // Flip the operator: the span no longer matches its stored hash.
    write(
        tmp.path(),
        "src/m.rs",
        "pub fn add(a: i64, b: i64) -> i64 { a - b }\n",
    );
    surf(tmp.path())
        .arg("check")
        .assert()
        .failure()
        .stdout(predicate::str::contains("DIVERGED"))
        .stdout(predicate::str::contains("1 divergence(s)"));
}

#[test]
fn check_malformed_hub_exits_one() {
    // A frontmatter typo is unenforceable — the gate must fail closed, not pass silently (#35).
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "surf.toml", "");
    write(
        tmp.path(),
        "hubs/a.md",
        "---\nsummary: x\nanchors:\n  - claim: [unterminated\n---\n",
    );
    surf(tmp.path())
        .arg("check")
        .assert()
        .failure()
        .stdout(predicate::str::contains("invalid hub"));
}

#[test]
fn lint_block_exits_one() {
    // A vanished symbol is a blocking lint finding.
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "surf.toml", "");
    write(tmp.path(), "src/m.rs", "pub fn other() {}\n");
    write(
        tmp.path(),
        "hubs/a.md",
        "---\nsummary: x\nanchors:\n  - claim: c\n    at: src/m.rs > ghost\n---\n",
    );
    surf(tmp.path())
        .arg("lint")
        .assert()
        .failure()
        .stdout(predicate::str::contains("error"));
}

#[test]
fn no_workspace_errors() {
    // No surf.toml anywhere up the tree → discovery fails, non-zero exit, message on stderr.
    let tmp = tempfile::tempdir().unwrap();
    surf(tmp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("no surf.toml"));
}

// ---- JSON contract: stdout parses and carries the versioned schema --------------------------

#[test]
fn check_json_contract() {
    let tmp = tempfile::tempdir().unwrap();
    clean_workspace(tmp.path());
    write(
        tmp.path(),
        "src/m.rs",
        "pub fn add(a: i64, b: i64) -> i64 { a - b }\n",
    );
    let out = surf(tmp.path())
        .args(["check", "--format", "json"])
        .assert()
        .failure()
        .get_output()
        .clone();

    let json = stdout_json(&out);
    assert_eq!(json["version"], REPORT_VERSION);
    let d = json["divergences"][0].as_object().unwrap();
    for key in [
        "hub", "claim", "at", "kind", "old_hash", "new_hash", "new_code", "prose",
    ] {
        assert!(d.contains_key(key), "missing key `{key}` in {d:?}");
    }
    assert_eq!(d["kind"], "changed");
}

#[test]
fn check_json_stdout_stays_pure_when_globs_warn() {
    // A zero-match --files glob warns on stderr; stdout must remain parseable JSON (the report
    // is consumed by machines, so the two streams cannot be mixed).
    let tmp = tempfile::tempdir().unwrap();
    clean_workspace(tmp.path());
    let out = surf(tmp.path())
        .args(["check", "--format", "json", "--files", "no/such/*.rs"])
        .assert()
        .failure() // a glob that scopes the gate to nothing must not read as clean (#78)
        .stderr(predicate::str::contains("matched no anchored files"))
        .get_output()
        .clone();
    let json = stdout_json(&out);
    assert_eq!(json["version"], REPORT_VERSION);
}

#[test]
fn lint_json_contract() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "surf.toml", "");
    write(tmp.path(), "src/m.rs", "pub fn other() {}\n");
    write(
        tmp.path(),
        "hubs/a.md",
        "---\nsummary: x\nanchors:\n  - claim: c\n    at: src/m.rs > ghost\n---\n",
    );
    let out = surf(tmp.path())
        .args(["lint", "--format", "json"])
        .assert()
        .failure()
        .get_output()
        .clone();

    let json = stdout_json(&out);
    let f = json.as_array().unwrap()[0].as_object().unwrap();
    for key in ["severity", "hub", "at", "message", "claim"] {
        assert!(f.contains_key(key), "missing key `{key}` in {f:?}");
    }
    assert_eq!(f["severity"], "block");
}

#[test]
fn verify_json_contract() {
    // An unstamped anchor: verify stamps it and reports the outcome as JSON.
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "surf.toml", "");
    write(
        tmp.path(),
        "src/m.rs",
        "pub fn add(a: i64, b: i64) -> i64 { a + b }\n",
    );
    write(
        tmp.path(),
        "hubs/a.md",
        "---\nsummary: x\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n---\n",
    );
    let out = surf(tmp.path())
        .args(["verify", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .clone();

    let json = stdout_json(&out);
    assert_eq!(json["stamped"], 1);
    let a = json["anchors"][0].as_object().unwrap();
    for key in ["hub", "at", "outcome"] {
        assert!(a.contains_key(key), "missing key `{key}` in {a:?}");
    }
    assert_eq!(a["outcome"], "stamped");
}

#[test]
fn stats_json_contract() {
    // stats reads git history, so the workspace must be a repo with at least one commit.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    git(root, &["init", "-q", "-b", "main"]);
    clean_workspace(root);
    commit_all(root, "seed");

    let out = surf(root)
        .args(["stats", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .clone();

    let json = stdout_json(&out);
    assert_eq!(json["version"], REPORT_VERSION);
    for key in ["commits", "rubber_stamp", "in_place"] {
        assert!(json.get(key).is_some(), "missing key `{key}` in {json:?}");
    }
    // Each rate carries n/d (and a nullable rate) — the shape downstream layers read.
    for key in ["n", "d", "rate"] {
        assert!(
            json["rubber_stamp"].get(key).is_some(),
            "missing rate key `{key}`"
        );
    }
}
