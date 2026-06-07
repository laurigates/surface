# Surface documentation

The map of Surface's docs. As content grows it lands in this tree rather than the README; a
dedicated docs site will later mirror this structure. For the pitch and a quick look, start with
the [README](../README.md).

> Reference material (commands, configuration, the hashing mechanism) currently lives in the
> README and is linked below. It will migrate into dedicated `docs/reference/` pages when the
> docs site is built — for now the README stays the single source of truth, to avoid two copies
> drifting apart.

## Overview
- [What Surface is and the problem it solves](../README.md#the-problem)
- [What it does that tests don't](../README.md#what-surface-does-that-tests-dont)
- [Is Surface for you?](../README.md#is-surface-for-you)

## Getting started
- [Install](../README.md#install)
- [Quickstart](../README.md#quickstart) — `init` → `new` → `lint` → `check` → `verify`

## Guides
- [Authoring hubs](./guides/authoring-hubs.md) — writing good claims and anchors, choosing
  granularity, and the verify loop.
- [CI integration](./guides/ci-integration.md) — the GitHub Action, the pre-commit hook, and
  scoping the gate to a PR.
- [Examples](./examples.md) — a minimal worked hub in each supported language.

## Reference *(currently in the README)*
- [Commands](../README.md#commands)
- [Configuration](../README.md#configuration)
- [How the gate works (technical) + the JSON report](../README.md#how-the-gate-works-technical)

## Concepts
- [What Surface does NOT do](../README.md#what-surface-does-not-do) — the honest limits.
- [The honest limit (§7) and the claim-maintenance risk (§8)](./surface-proposal.md) — in the spec.

## Development *(internal — not part of the published site)*
- [Product spec / proposal](./surface-proposal.md) — the source of the `§` references in hubs.
- [Build phases](./phases/README.md) — how the MVP was built, one file per phase.
- [Contributing](../CONTRIBUTING.md) — build, test, dogfood.
- [AGENTS.md](../AGENTS.md) — on-ramp for AI coding agents.
