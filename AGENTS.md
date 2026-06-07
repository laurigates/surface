# AGENTS.md

Guidance for AI coding agents working in this repo. (Humans: see
[`CONTRIBUTING.md`](./CONTRIBUTING.md) and [`docs/`](./docs/index.md).)

Surface is a deterministic gate that surfaces divergence between docs and code: you anchor a
sentence to the code it describes, and `surf check` blocks when that code's logic changes out
from under the prose. **This repo dogfoods Surface on its own source** — the gate runs on
`surf-core`/`surf-cli`.

## Where the context lives: `hubs/`

Read [`hubs/`](./hubs/) first. Each hub is markdown prose describing an invariant of this
codebase, with frontmatter anchoring the claim to a specific symbol. Unlike code comments,
**hub prose is sealed by `surf check`** — if the anchored code changed since a human last
confirmed the prose, the gate fails. So the hubs are trustworthy and current in a way comments
are not. They are the fastest accurate way to understand how a part of the system works.

Caveat (the tool's own honest limit): a green gate means *the anchored code hasn't changed
since last verified* — not that every sentence is true, and nothing about code no hub anchored.
If you read a hub claim, sanity-check it against the code it points at before relying on it.

## When you add or change a feature — keep the hubs honest

Run the loop (binary builds to `target/debug/surf`; see `CONTRIBUTING.md` for build commands):

1. Make the change.
2. `surf lint` — every anchor must resolve. Consider the advisory granularity warnings
   (over/under-anchoring); they are nudges, not blocks.
3. `surf check` — if you touched code a hub anchors, it will report `DIVERGED`. Re-read the
   claim. If the prose **still holds**, `surf verify` re-seals it (writes the new hash); if the
   prose is **now false**, fix the prose first, then verify.
4. Added public behavior? Add a hub claim for it — the under-coverage warning flags public
   functions with no claim. When you update a hub, update its *prose* to stay accurate, not just
   the hash.
5. Record user-facing changes in [`CHANGELOG.md`](./CHANGELOG.md) under `[Unreleased]`.

Do not blindly `surf verify` to make the gate green — that is the rubber-stamping failure the
tool exists to prevent. Verify means "I read the prose and it is still true."

## Pointers

- [`hubs/`](./hubs/) — governed context for this codebase (read first).
- [`CHANGELOG.md`](./CHANGELOG.md) — what changed; update `[Unreleased]`.
- [`docs/index.md`](./docs/index.md) — documentation map (guides, reference, concepts).
- [`CONTRIBUTING.md`](./CONTRIBUTING.md) — build, test, format, lint commands and layout.
- [`docs/surface-proposal.md`](./docs/surface-proposal.md) — the product spec (the `§` references in hubs).
