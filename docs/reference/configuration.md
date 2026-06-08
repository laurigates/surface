---
title: Configuration
description: surf.toml marks the workspace and globs your hubs; the supported languages; and what the gate needs from CI.
---

A `surf.toml` at the repo root marks the workspace — `surf` walks up from the current directory to
find it, like `git` or `ruff` — and globs your hubs:

```toml
hubs = ["hubs/*.md"]
```

Point the glob wherever your hubs live: keep them central (`hubs/*.md`) or co-locate them with code
(e.g. `["**/_hub.md"]`).

## Languages

TypeScript (`.ts`, `.tsx`, `.mts`, `.cts`), JavaScript/JSX (`.js`, `.jsx`, `.mjs`, `.cjs`), Rust
(`.rs`), Python (`.py`, `.pyi`), and Go (`.go`). Grammars are compiled into the binary and
version-pinned, so a hash computed on your laptop and in CI always agree.

In Python, `at:` resolves callables (functions, methods, classes) **and** non-callables — module
constants, type aliases (`X = Literal[...]`, `type X = ...`), and class attributes
(`Class > attr`).

## CI

The gate hashes your working tree and compares it to the hash committed in the frontmatter. It
needs the checkout, **not** the history — do **not** set `fetch-depth: 0`. (The advisory
`old_code` / `magnitude` use a single `git show` of the base ref; with no git available the verdict
is unchanged, those fields are just omitted.) See [CI integration](../guides/ci-integration.md).
