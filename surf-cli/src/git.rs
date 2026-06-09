//! Best-effort git queries. The deterministic verdict never depends on these: they scope and
//! enrich `check` (advisory `old_code`/`magnitude`) and let `lint`/`verify` recognize a moved
//! file. Every function returns `None`/empty when git can't answer (no repo, bad ref, shallow
//! clone), so the gate degrades to a full, git-free check rather than failing.

use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// Files changed between the merge base of `base`..HEAD and the working tree. Paths are
/// repo-root-relative; they match `Anchor.file` (workspace-root-relative) when the workspace
/// root is the repo root, the normal case. `None` if git can't answer.
pub fn changed_files(root: &Path, base: &str) -> Option<HashSet<String>> {
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

/// Commit SHAs (newest first) in the optional `since`/`until` window, merges excluded so each
/// SHA is one unit of work (`surf stats` treats a commit as a PR). `None` if git can't answer.
pub fn log_commits(root: &Path, since: Option<&str>, until: Option<&str>) -> Option<Vec<String>> {
    let mut args: Vec<String> = vec!["log".into(), "--no-merges".into(), "--format=%H".into()];
    if let Some(s) = since {
        args.push(format!("--since={s}"));
    }
    if let Some(u) = until {
        args.push(format!("--until={u}"));
    }
    let output = Command::new("git")
        .current_dir(root)
        .args(&args)
        .output()
        .ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_string)
            .collect()
    })
}

/// Repo-root-relative paths changed by a single commit (vs its first parent). Empty for the root
/// commit's tree-only diff is fine. `None` if git can't answer.
pub fn commit_files(root: &Path, sha: &str) -> Option<Vec<String>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["diff-tree", "--no-commit-id", "--name-only", "-r", sha])
        .output()
        .ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_string)
            .collect()
    })
}

/// Every tracked file at `sha` (repo-root-relative). Used to find the hub set as it existed at a
/// past commit. `None` if git can't answer.
pub fn list_files_at(root: &Path, sha: &str) -> Option<Vec<String>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-tree", "-r", "--name-only", sha])
        .output()
        .ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_string)
            .collect()
    })
}

/// The contents of `rel_file` at `base` (e.g. `git show HEAD:src/x.rs`). `None` if unavailable.
pub fn show(root: &Path, base: &str, rel_file: &str) -> Option<String> {
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

/// The path `old_path` was renamed to, per git's rename detection between HEAD and the working
/// tree. `None` if git can't answer or no rename pairs with `old_path`. Best-effort: a pure
/// `mv` without a content match may show as delete+add and not be detected.
pub fn renamed_to(root: &Path, old_path: &str) -> Option<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-status", "--find-renames", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        // Rename rows are `R<score>\t<old>\t<new>`.
        let mut parts = line.split('\t');
        let Some(status) = parts.next() else {
            continue;
        };
        if !status.starts_with('R') {
            continue;
        }
        let (Some(old), Some(new)) = (parts.next(), parts.next()) else {
            continue;
        };
        if old == old_path {
            return Some(new.to_string());
        }
    }
    None
}
