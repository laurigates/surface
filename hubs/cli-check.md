---
summary: surf check — the gate. Hash each anchored span, compare to the stored hash, block on divergence. Optionally scope to changed files.
anchors:
  - claim: >
      Per claim: resolve and hash every site, combine into one hash, compare to the stored
      hash. No stored hash → Unverified; an anchor that no longer resolves → Unresolvable;
      a mismatch → Changed. The verdict is deterministic and needs no git.
    at: surf-cli/src/check.rs > check_claim
    hash: e04e680e6d8b
  - claim: >
      Scoping is opt-in and intersective: with neither --base nor --files every claim is checked.
      A claim is in scope when any of its anchored files matches each active filter — the --base
      changed-files set (merge-base..working-tree) and/or the --files globs. A bad ref or non-repo
      yields no changed set, falling back to a full check rather than checking nothing. Each glob
      records whether it ever matched an anchored file (tallied before the --base filter), so a
      pattern that scopes the gate to nothing is detectable after the walk.
    at: surf-cli/src/check.rs > Scope > includes
    hash: f18aefc5097e
  - claim: >
      The gate fails closed: a hub whose frontmatter won't parse yields an Unresolvable
      divergence (blocking the run) rather than being silently skipped, so a frontmatter typo
      can't pass as clean. Alongside the divergences it returns the --files patterns that
      matched no anchored file; run warns on stderr for each and exits non-zero when every
      pattern matched nothing, so a typo'd --files can't read as a clean run.
    at: surf-cli/src/check.rs > check_workspace
    hash: 567ba4ebe18e
refs: []
---

# surf check

`check_claim` is the verdict; the git helpers in [`cli-git.md`](./cli-git.md) only feed the
advisory `old_code`/`magnitude` in the `--format json` report. Any divergence makes `run` exit
non-zero (the CI-blocking signal). `Scope` narrows which claims `check_workspace` evaluates when
`--base`/`--files` are given.
