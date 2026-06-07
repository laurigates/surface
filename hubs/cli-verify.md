---
summary: surf verify — re-seal a claim after a human confirms the prose, with optional --follow.
anchors:
  - claim: >
      For each claim, plan_claim re-hashes every site (combined) when all resolve, returning
      Unchanged when that hash already matches the stored one or Hash to re-stamp otherwise;
      if a site fails to resolve it re-points a renamed single-segment anchor via find_renamed
      under --follow, else skips with a reason. It never edits prose, only the hash/at line.
    at: surf-cli/src/verify.rs > plan_claim
    hash: b44fea4ee5a8
refs: []
---

# surf verify

The human escape hatch. `verify_all` applies each `plan_claim` result through the surgical hub
editor and only rewrites a file when something actually changed; `run` then renders the
collected report as human text or JSON.
