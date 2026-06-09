---
summary: surf for — reverse lookup of hubs/claims anchored into a file; read-only query.
anchors:
  - claim: >
      run normalizes the queried path to workspace-root-relative form, finds the matching claims,
      and prints them grouped by hub (human) or as a versioned {version, path, matches} envelope
      (JSON). It is a query, not a gate, so it always exits 0 whether or not anything matched.
    at: surf-cli/src/for_path.rs > run
    hash: 0e15525b1340
  - claim: >
      find collects every claim whose anchored file equals the queried path (matched on path only —
      no source parse), optionally narrowed to anchors whose first segment is the given symbol.
      Malformed hubs are skipped rather than erroring, and results are sorted by hub then anchor.
    at: surf-cli/src/for_path.rs > find
    hash: 047c1480c650
refs: []
---

# surf for

Delivers the discoverability half of the thesis: a fast way to pull up the claims governing a
file before touching its logic. `run` normalizes the queried path to workspace-root-relative form,
calls `find`, and prints matches grouped by hub (human) or as a versioned `{version, path,
matches}` envelope (JSON). No model, no network, no source parse — purely a read over the hub set.
