# Dogfood log

Raw, dated notes from using Surface on Surface (and on other repos). Not polished — this is
source material for write-ups later. One entry per notable moment: *what happened, what the tool
did about it, the lesson.* Keep it honest; the failures are the interesting part.

---

## 2026-06-11 — What an anchor can reach, and what it can't

**Context:** PR 3 of 0.6.0 (`#52`) — adding `surf suggest --all` to propose Python classes and
non-callables. It touched the shared `public_symbols` enumerator, the clap `Command` enum, and
`suggest.rs`.

**What happened — the reach:** `surf check` tripped on `hubs/cli-reference.md`, whose claim is
anchored to the `Command` enum and whose prose *literally ends with an instruction to me*:

```
... Adding, removing, or renaming a command or flag, or changing a default, diverges this
anchor — re-read docs/reference/commands.md before sealing.
```

`docs/reference/commands.md` is a hand-written human doc with **no anchor of its own** — nothing
hashes it, so on its own it could rot freely. But because the *source of truth* (the clap enum)
is anchored, and the claim encodes the cross-reference, adding `--all` forced the gate red until
I went and updated that un-anchored sibling doc. An anchor on the thing that changes, used as a
tripwire for the prose that describes it elsewhere. That's a pattern worth naming: you don't have
to anchor the downstream doc, you anchor its *cause* and write the pointer into the claim.

**What happened — the blind spot:** PR 2 had just re-pointed `lint`'s coverage nudge at
`public_symbols`. In PR 3 I broadened `public_symbols` — and if I'd broadened its *default*
instead of gating the new kinds behind `--all`, `lint` would have started flagging every
unanchored class and constant in every repo. The gate could **not** have caught that: no hash
changes, no anchored span moves — it's a semantic coupling between two callers of a shared
function. I had to hold it in my head and design around it. Nothing in Surface protects you from
it.

**Why it's a good story:** the two halves are a clean contrast. The gate's reach is longer than
"the span you anchored" — via an instruction in the claim it pulled an un-anchored doc into
scope. But its blind spot is equally real: behaviour that emerges from how two functions share a
third is invisible to a per-span hash. Anchor the cause, not just the symbol — and don't expect
the gate to catch coupling it can't see.

**Lesson / open question:** the `commands.md` trick (anchor the source of truth, point at the
prose) generalizes — is it worth documenting as an authoring pattern? And the blind spot is the
honest counterweight to the PR 1 entry's "what's anchored is enforced": *what's anchored is
enforced span-locally; cross-symbol invariants still live only in your head.*

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
