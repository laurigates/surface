---
summary: surf lint — every anchor resolves to exactly one symbol; renames warn, not block.
anchors:
  - claim: >
      lint produces a Finding per anchor site: ambiguous or vanished anchors block, while a
      renamed-but-present symbol (stored-hash match) only warns and points at verify --follow.
      Block-level findings set a non-zero exit; warnings alone keep exit 0.
    at: surf-cli/src/lint.rs > lint_site
    hash: 840a2d93cf22
refs: []
---

# surf lint

`lint_workspace` loads every hub and runs `lint_site` over each anchor; `run` prints the
findings and chooses the exit code.
