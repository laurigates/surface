---
summary: surf suggest — propose anchors for unanchored public symbols; read-only, never stamps.
anchors:
  - claim: >
      surf suggest is read-only: run scans the given globs, lists each public symbol no hub
      already anchors, and prints them (a starter hub in human mode, or JSON). By default the
      surface is callables (top-level functions plus Python/Go methods); --all additionally
      proposes the non-callable targets resolve accepts — Python top-level classes, module-level
      constants and type aliases, and class attributes; Go exported const/var/type declarations.
      It warns on stderr for any glob that matched no files, notes when --all scanned Rust or
      TypeScript files it cannot affect, and exits non-zero only when every glob was empty. It
      never writes a file and never computes or stamps a hash — the author edits the claims and
      verifies.
    at: surf-cli/src/suggest.rs > run
    hash: 9f598cd3a7b9
refs: []
---

# surf suggest

Lowers the §8 authoring cost. `run` reads existing hub coverage, scans the requested source
globs via `scan` (which reuses `surf_core::public_symbols` and skips already-anchored symbols,
keyed on the full `file > seg > seg` anchor path so anchoring one method doesn't hide its
siblings), and prints suggestions. Per-glob match tallies let a typo'd glob read differently from
a clean "all anchored". Suggestions only: the author turns them into real claims and runs
`surf verify`.
