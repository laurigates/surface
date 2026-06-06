# Surface MVP — Phases

Phased build of the Surface MVP. Read [`OVERVIEW.md`](./OVERVIEW.md) first — it carries the
context, locked decisions, and scope guardrails every phase assumes. The product spec is
[`../surface-proposal.md`](../surface-proposal.md).

Each phase file is self-contained: **Goal**, **Proposal refs**, **Depends on**, **Steps**,
**Files touched**, **Verify**, **Status**. Work them in dependency order; update the Status
line as you go.

## Status tracker

| Phase | File | Status |
|---|---|---|
| 0 | [`00-toolchain-scaffold.md`](./00-toolchain-scaffold.md) | done |
| 1 | [`01-anchor-resolution.md`](./01-anchor-resolution.md) | done |
| 2 | [`02-canonical-hashing.md`](./02-canonical-hashing.md) | not started |
| 3 | [`03-hub-format.md`](./03-hub-format.md) | not started |
| 4 | [`04-surf-lint.md`](./04-surf-lint.md) | not started |
| 5 | [`05-surf-check.md`](./05-surf-check.md) | not started |
| 6 | [`06-surf-verify.md`](./06-surf-verify.md) | not started |
| 7 | [`07-distribution.md`](./07-distribution.md) | not started |

## Locked decisions (see OVERVIEW for rationale)
- **Grammars:** TypeScript + Rust (Surface dogfoods its own Rust source).
- **Layout:** Cargo workspace — `surf-core` (pure, no I/O) + `surf-cli` (clap binary).
- **Rust install:** `rustup` via `curl | sh`, toolchain pinned in `rust-toolchain.toml`.

## Scope guardrails — NOT in the MVP
- No `refs` resolver, `surf index`, MCP service, reviewer plugin, or `covers` field.
- No similarity-score gating. Boolean AST hash only; tree-edit magnitude is advisory JSON.
- Gate promises only "named span unchanged since last verified" — never system-wide invariants.
