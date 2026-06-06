---
summary: Supported languages, file-extension detection, and bundled tree-sitter grammars.
anchors:
  - claim: >
      Language is detected purely by file extension (ts/tsx/mts/cts, rs, py/pyi, go); an
      unknown extension yields None and the anchor is treated as unsupported.
    at: surf-core/src/lang.rs > Lang > from_path
    hash: 0a9fa1d91eeb
refs: []
---

# Languages

`Lang` maps extensions to a bundled, version-pinned tree-sitter grammar. Adding a language is
additive: one `Lang` variant, an extension arm, a grammar, and a `Family`.
