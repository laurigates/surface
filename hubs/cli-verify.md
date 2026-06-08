---
summary: surf verify — re-seal a claim after a human confirms the prose, with optional --follow.
anchors:
  - claim: >
      For each claim, plan_claim re-hashes every site (combined) when all resolve, returning
      Unchanged when that hash already matches the stored one or Hash to re-stamp otherwise.
      Under --follow, a site that no longer resolves re-points a renamed single-segment anchor
      via find_renamed; a site whose file is unreadable asks git where it moved and re-points the
      path (only when the code is otherwise unchanged). Otherwise it skips with a reason. It
      never edits prose, only the hash/at line.
    at: surf-cli/src/verify.rs > plan_claim
    hash: 6de72f5412b9
refs: []
---

# surf verify

The human escape hatch. `verify_all` applies each `plan_claim` result through the surgical hub
editor and only rewrites a file when something actually changed; `run` then renders the
collected report as human text or JSON.
