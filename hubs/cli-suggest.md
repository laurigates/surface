---
summary: surf suggest — propose anchors for unanchored public functions; read-only, never stamps.
anchors:
  - claim: >
      surf suggest is read-only: run scans the given globs, lists each top-level public function
      no hub already anchors, and prints them (a starter hub in human mode, or JSON). It never
      writes a file and never computes or stamps a hash — the author edits the claims and verifies.
    at: surf-cli/src/suggest.rs > run
    hash: cf02ef9af242
refs: []
---

# surf suggest

Lowers the §8 authoring cost. `run` reads existing hub coverage, scans the requested source
globs via `scan` (which reuses `surf_core::public_fns` and skips already-anchored symbols), and
prints suggestions. Suggestions only: the author turns them into real claims and runs
`surf verify`.
