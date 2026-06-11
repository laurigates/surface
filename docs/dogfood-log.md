# Dogfood log

Raw, dated notes from using Surface on Surface (and on other repos). Not polished — this is
source material for write-ups later. One entry per notable moment: *what happened, what the tool
did about it, the lesson.* Keep it honest; the failures are the interesting part.

---

## 2026-06-11 — The gate caught its own author lying

**Context:** Implementing PR 1 of the 0.6.0 milestone (`#53` + `#38`) — making `surf for`,
`surf check --files`, and `surf stats` fail loudly on malformed input instead of returning a
falsely-reassuring success.

**What happened:** After editing `for_path.rs` and `stats.rs`, I ran the repo's own gate
(`surf check`) as the final verification step. It failed — on Surface's *own* anchored claims:

```
DIVERGED  hubs/cli-for.md :: surf-cli/src/for_path.rs > run
    claim: ... It is a query, not a gate, so it always exits 0 whether or not anything matched.
```

That claim had been **true at 0.5.0 and was now false** — the whole point of `#53` was to make
`for` exit 1 on a mistyped path. The change to the *behavior* and the change to the *documented
contract* were the same act, and the gate refused to let them diverge silently. I couldn't
re-seal the hash without first deciding: is the prose still true? It wasn't, so I rewrote it.

Three claims tripped (`cli-for`, `cli-stats`, `cli-check`). Two were genuine contract changes
that needed new prose; one (`check_workspace`) only shifted because an adjacent line moved, so it
just needed re-sealing. The tool made me look at all three and tell them apart by hand — which is
exactly the discrimination it's supposed to force.

**Why it's a good story:** the usual pitch for docs-as-tests is abstract ("docs drift from
code"). This is the concrete version, and it's self-referential: the gate caught *its own
maintainer*, mid-feature, shipping a behavior change that contradicted a sentence the tool itself
was responsible for guarding. The stale module doc comment in `for_path.rs` (`// A query, not a
gate: it always exits 0`) was **not** anchored — so it drifted with zero resistance, and I only
caught it by eye. A nice illustration of the boundary: what's anchored is enforced; what isn't,
rots.

**Lesson / open question:** the un-anchored doc comment drifting while the anchored claim held is
the sharpest line in the whole episode. Worth a callout in any write-up: coverage is the product.
Possible follow-on — should `lint` nudge toward anchoring module-level doc comments that restate
a contract? (Adjacent to `#54`, the coverage-nudge work.)

---

<!-- New entries above this line, newest first. Template:

## YYYY-MM-DD — One-line hook

**Context:** what you were doing.
**What happened:** the moment, with the real command/output if you have it.
**Why it's a good story:** the angle a reader would care about.
**Lesson / open question:** what it changed or what it leaves open.
-->
