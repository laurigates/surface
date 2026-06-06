---
summary: surf verify — re-seal a claim after a human confirms the prose, with optional --follow.
anchors:
  - claim: >
      For each claim, plan_claim re-hashes every site (combined) when all resolve; otherwise,
      under --follow, it re-points a renamed single-segment anchor via find_renamed and
      re-hashes; otherwise it skips with a reason. It never edits prose, only the hash/at line.
    at: surf-cli/src/verify.rs > plan_claim
    hash: 6a3131f27a77
refs: []
---

# surf verify

The human escape hatch. `run` applies each `plan_claim` result through the surgical hub editor
and only rewrites a file when something actually changed.
