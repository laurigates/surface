# Phase 6 — `surf verify`

**Goal:** the human escape hatch (§8) — re-hash after a human confirms the prose still
holds. `--follow` re-points a renamed anchor and re-hashes in one step.

**Proposal refs:** §8 (escape hatch, "I looked, still true"), §6.4 (`--follow` for renames), §9.1.4.

**Depends on:** Phase 5.

**Status:** not started

## Steps

1. `surf verify [selector]` (hub / anchor selector): re-resolve (P1), re-hash (P2), and
   **write the new `hash` back into the frontmatter** — the explicit "I looked, still true".
   With symbol-scoped AST anchoring this should be needed only *occasionally*; running it on
   most PRs is a smell that anchors are too coarse (§8).
2. `--follow`: when the symbol was renamed, update the `at:` path to the new
   name/location, then re-hash — clearing the Phase 4 rename warning in one step (§6.4).
3. Preserve frontmatter key order / formatting on write as much as practical (minimize diff
   noise; authors will review these writes).

## Files touched
- `surf-cli/src/verify.rs`
- `surf-cli/src/main.rs` (wire subcommand + `--follow`)
- frontmatter writer (in `surf-core/src/hub.rs`, paired with the Phase 3 parser)

## Verify
- Divergence → `surf verify <anchor>` → `surf check` now clean, and the diff is only the `hash` field.
- Rename → `surf verify --follow` updates `at:` to the new symbol and clears the lint warning.
- A no-op verify (span unchanged) leaves the file byte-identical.
