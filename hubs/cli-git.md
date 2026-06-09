---
summary: Best-effort git queries for scoping and rename-following — advisory only, the gate never depends on them.
anchors:
  - claim: >
      changed_files returns the repo-relative paths changed between the merge base of base..HEAD
      and the working tree, used to diff-scope the check. A missing merge base (shallow clone)
      falls back to diffing the ref directly; if git can't answer at all it returns None.
    at: surf-cli/src/git.rs > changed_files
    hash: 9f422d548239
  - claim: >
      show returns the contents of a file at a git ref (git show <base>:<path>), used to recover
      the previous source for advisory old_code/magnitude. None when unavailable — the verdict is
      unchanged either way.
    at: surf-cli/src/git.rs > show
    hash: 6398bf958ad1
  - claim: >
      renamed_to asks git's rename detection (diff --name-status --find-renames HEAD) for the new
      path a file moved to, letting lint warn and verify --follow re-point instead of hard-blocking.
      Best-effort: a pure mv with no content match may show as delete+add and not be detected, and
      None means git couldn't pair the rename — the deterministic verdict never depends on it.
    at: surf-cli/src/git.rs > renamed_to
    hash: 9622170a3b9a
  - claim: >
      log_commits lists commit SHAs (newest first) in the optional since/until window with merges
      excluded, so surf stats treats each SHA as one unit of work. None when git can't answer.
    at: surf-cli/src/git.rs > log_commits
    hash: 748b15a706c2
  - claim: >
      commit_files lists the repo-relative paths a single commit changed versus its first parent
      (diff-tree --no-commit-id --name-only -r). None when git can't answer.
    at: surf-cli/src/git.rs > commit_files
    hash: cef27873a3e1
  - claim: >
      list_files_at lists every tracked file at a commit (ls-tree -r --name-only), used to find the
      hub set as it existed at a past commit. None when git can't answer.
    at: surf-cli/src/git.rs > list_files_at
    hash: cbe066de9432
refs: []
---

# git helpers

A thin, best-effort wrapper over `git` via `std::process::Command` — no `git2` dependency. Every
function degrades to `None`/empty when git can't answer (no repo, bad ref, shallow clone), so the
gate stays deterministic and git-free: these only *enrich* `check` and let `lint`/`verify`
recognize a moved file ([`rename.md`](./rename.md) covers symbol renames; `renamed_to` covers the
file-rename case).
