---
summary: surf stats — git-history adoption metrics (rubber-stamp + in-place rates); advisory, never a gate.
anchors:
  - claim: >
      run computes the two metrics and prints them human-readable or as a versioned envelope; it
      always exits 0 on success and surfaces an error (non-zero) only when git history is
      unavailable. The metrics are advisory and never gate.
    at: surf-cli/src/stats.rs > run
    hash: 7f4ab96fac92
  - claim: >
      compute walks each non-merge commit in the since/until window. A rubber-stamp event is an
      already-sealed claim whose stored hash value changed in a commit; it counts toward the
      rubber-stamp numerator only when the claim's prose was unchanged. A claim-touch event is a
      commit that changed a file the claim anchors; it counts toward the in-place numerator when
      the claim's stored hash was updated in that same commit. Claim identity is its at: site(s),
      and missing git history or an invalid hub glob in surf.toml is a hard error rather than a
      silent zero or a quietly-narrowed hub set.
    at: surf-cli/src/stats.rs > compute
    hash: 4adab21bd94e
refs: ["../docs/guides/stats.md"]
---

# surf stats

The proposal's adopt/kill signals (§9.2), computed deterministically from git history. `compute`
parses the hub set as it existed at each commit (and its parent) via `git ls-tree`/`git show`,
compares stored hashes and prose across the pair, and tallies the two rates. Heuristics — one
commit per PR, `at:`-site claim identity, an in-place denominator that counts any anchored-file
edit — are documented in [the stats guide](../docs/guides/stats.md).
