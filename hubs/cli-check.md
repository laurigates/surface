---
summary: surf check — the gate. Hash each anchored span, compare to the stored hash, block on divergence. Optionally scope to changed files.
anchors:
  - claim: >
      Per claim: resolve and hash every site, combine into one hash, compare to the stored
      hash. No stored hash → Unverified; an anchor that no longer resolves → Unresolvable;
      a mismatch → Changed. The verdict is deterministic and needs no git.
    at: surf-cli/src/check.rs > check_claim
    hash: c1eed8d5f41b
  - claim: >
      Scoping is opt-in and intersective: with neither --base nor --files every claim is checked.
      A claim is in scope when any of its anchored files matches each active filter — the --base
      changed-files set (merge-base..working-tree) and/or the --files globs. A bad ref or non-repo
      yields no changed set, falling back to a full check rather than checking nothing.
    at: surf-cli/src/check.rs > Scope > includes
    hash: 2e21db33542d
refs: []
---

# surf check

`check_claim` is the verdict; the git helpers in [`cli-git.md`](./cli-git.md) only feed the
advisory `old_code`/`magnitude` in the `--format json` report. Any divergence makes `run` exit
non-zero (the CI-blocking signal). `Scope` narrows which claims `check_workspace` evaluates when
`--base`/`--files` are given.
