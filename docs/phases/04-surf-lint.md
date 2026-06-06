# Phase 4 — `surf lint`

**Goal:** frontmatter is well-formed; every `at:` resolves to **exactly one** node;
renames are handled per §6.4. Composes Phases 1–3.

**Proposal refs:** §9.1.2 (lint), §6.3 (exactly-one resolution), §6.4 (renames are an MVP problem), §11.3 (granularity guidance — minimal here).

**Depends on:** Phases 1, 3.

**Status:** not started

## Steps

1. Discover `surf.toml`, load hubs via the glob, parse frontmatter (Phase 3).
2. For each anchor, resolve via Phase 1:
   - `Ambiguous` → error, tell the author to add `@N`.
   - `NotFound` → see rename handling below.
3. **Rename handling (§6.4)** — hits the MVP constantly, not deferrable:
   - **Renamed but clearly present** (git rename detection + high AST similarity of the
     moved node via the Phase 2 magnitude) → **warn**, not block; suggest
     `surf verify --follow` to re-point and re-hash in one step.
   - **Genuinely vanished** (no plausible match) → **block**: a claim now points at nothing.
4. Human-readable diagnostics (which hub, which claim, which `at:`). Non-zero exit on any
   block-level finding; warnings alone keep exit 0.

## Files touched
- `surf-cli/src/lint.rs`
- `surf-cli/src/main.rs` (wire subcommand)
- git rename detection helper (shell out to `git` or a plumbing call) — likely `surf-core` or a cli helper

## Verify
Fixture hub + fixture source covering each case:
- clean → exit 0, no findings.
- ambiguous (collision, no `@N`) → block, message names the fix.
- vanished symbol → block.
- renamed symbol → **warn** (exit 0), message suggests `verify --follow`.
Assert exit codes for each.
