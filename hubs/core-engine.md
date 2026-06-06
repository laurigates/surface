---
summary: How Surface resolves an anchor to a span and hashes it deterministically.
anchors:
  - claim: >
      Anchor resolution treats a scope as a set of nodes, so a type and its impl
      (which share a name) both get descended — `Type > method` is unique even when
      `Type` alone is ambiguous.
    at: surf-core/src/resolve.rs > resolve_node
    hash: 4f8bbae191ac
  - claim: >
      The canonical hash is quiet on consistent renames (identifiers are alpha-renamed
      to positional placeholders) but loud on operators, keywords, and literal values.
    at:
      - surf-core/src/hash.rs > emit
      - surf-core/src/hash.rs > is_identifier
    hash: 125a2640f019
refs: []
---

# Core engine

This hub documents the two load-bearing pieces of `surf-core`: turning an `at:` anchor
into an exact symbol span, and hashing that span so the gate fires on the *right* change.

Hashes are intentionally absent until `surf verify` stamps them — authoring a claim and
confirming it are separate acts.
