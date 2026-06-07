---
summary: surf check — the gate. Hash each anchored span, compare to the stored hash, block on divergence. Optionally scope to changed files.
anchors:
  - claim: >
      Per claim: resolve and hash every site, combine into one hash, compare to the stored
      hash. No stored hash → Unverified; an anchor that no longer resolves → Unresolvable;
      a mismatch → Changed. The verdict is deterministic and needs no git.
    at: surf-cli/src/check.rs > check_claim
    hash: eaa9b62224f4
  - claim: >
      old_code and magnitude are best-effort enrichment recovered from the previous source via
      `git show <base>:<path>`; absent git the verdict is unchanged and those fields are omitted.
    at: surf-cli/src/check.rs > git_show
    hash: cd1f35beb1ec
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

`check_claim` is the verdict; `git_show` only feeds the advisory `old_code`/`magnitude` in the
`--format json` report. Any divergence makes `run` exit non-zero (the CI-blocking signal).
`Scope` narrows which claims `check_workspace` evaluates when `--base`/`--files` are given.
