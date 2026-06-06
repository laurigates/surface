---
summary: Resolving an anchor to the exact span of one symbol, across language families.
anchors:
  - claim: >
      The generic resolver treats a scope as a *set* of nodes, so a type and its impl/methods
      (which share a name) both get descended — `Type > method` is unique even when `Type`
      alone is ambiguous. Resolves to exactly one node or returns NotFound/Ambiguous.
    at: surf-core/src/resolve.rs > resolve_node
    hash: 94f8e4052a2c
  - claim: >
      Go is resolved by a dedicated path: its symbols are flat (no nested declarations) and
      methods attach to a type by receiver, so `Type > Method` matches a method_declaration
      whose receiver type equals the type.
    at: surf-core/src/resolve.rs > resolve_go
    hash: 82b3f6e863cf
  - claim: >
      Rename detection enumerates every definition at any depth so a renamed-but-unchanged
      symbol can be found by hash.
    at: surf-core/src/resolve.rs > collect_all_defs
    hash: a75886451d68
refs: []
---

# Resolution

`resolve_node` is the load-bearing primitive: anchor + parsed tree → exact byte/line span.
TypeScript/Rust/Python use the generic scope-set walk; Go uses `resolve_go`.
