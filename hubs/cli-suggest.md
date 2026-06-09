---
summary: surf suggest — propose anchors for unanchored public symbols; read-only, never stamps.
anchors:
  - claim: >
      surf suggest is read-only: run scans the given globs, lists each public symbol (top-level
      function, plus Python/Go methods) no hub already anchors, and prints them (a starter hub in
      human mode, or JSON). It warns on stderr for any glob that matched no files, and exits
      non-zero only when every glob was empty. It never writes a file and never computes or stamps
      a hash — the author edits the claims and verifies.
    at: surf-cli/src/suggest.rs > run
    hash: 9f907f6299ff
refs: []
---

# surf suggest

Lowers the §8 authoring cost. `run` reads existing hub coverage, scans the requested source
globs via `scan` (which reuses `surf_core::public_symbols` and skips already-anchored symbols,
keyed on the full `file > seg > seg` anchor path so anchoring one method doesn't hide its
siblings), and prints suggestions. Per-glob match tallies let a typo'd glob read differently from
a clean "all anchored". Suggestions only: the author turns them into real claims and runs
`surf verify`.
