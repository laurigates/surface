---
summary: AST-canonical hashing — quiet on cosmetics, loud on logic — and per-claim combination.
anchors:
  - claim: >
      The canonical token stream drops comments, alpha-renames identifiers to positional
      placeholders (consistent rename → same tokens; swapping two names → different), and
      keeps operators, keywords, and literal values verbatim.
    at: surf-core/src/hash.rs > emit
    hash: 5ecc5e15a524
  - claim: >
      Identifier node kinds are enumerated per language family; only these are alpha-renamed,
      everything else (operators, keywords, literals) is kept.
    at: surf-core/src/hash.rs > is_identifier
    hash: ac8c69676a07
  - claim: >
      A claim's hash is the combination of its per-site hashes — a single site is the identity,
      multiple sites combine order-sensitively, so the claim is stale if any listed span changes.
    at: surf-core/src/hash.rs > combine_site_hashes
    hash: 83a72772c92d
refs: []
---

# Canonical hashing

The fingerprint is computed over `emit`'s token stream, hashed with SHA-256 (12 hex). This is
the signal the gate compares; `Magnitude` alongside it is advisory only and never gates.
