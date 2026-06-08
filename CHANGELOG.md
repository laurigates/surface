# Changelog

All notable changes to Surface are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-06-08

### Added
- Python: `at:` now resolves **non-callable** symbols — module-level constants and type aliases
  (`X = …`, `X: T = …`, PEP 695 `type X = …`) and class-level attributes (`Class > attr`,
  including annotation-only). Previously only functions, methods, and classes resolved (#28).
- Per-claim `ignore_literals: true` frontmatter option — excludes string-literal *content* from a
  claim's hash so a copy edit inside the anchored span no longer re-opens the gate. Logic edits
  (operators, numbers, structure) are still caught. The stored hash is computed in this mode, so
  it travels with the claim rather than a CLI flag (#21).
- `surf verify --follow` and `surf lint` now follow **file renames** via git rename detection:
  a moved file makes `lint` warn (and point at `--follow`) instead of hard-blocking, and
  `--follow` rewrites the anchor's path — only when the code is otherwise unchanged. Best-effort
  and git-dependent; the deterministic `surf check` verdict never depends on it (#3).

### Changed
- Python decorators are now part of an anchored function/class's hashed span, and a decorator's
  *name* is kept verbatim (not alpha-renamed) — so adding/removing a decorator, changing its
  arguments, or swapping it (`@cache` → `@lru_cache`, `@staticmethod` → `@classmethod`) trips the
  gate. Previously decorators were excluded from the span entirely (#8).
- **Breaking (JSON):** `surf check --format json` now emits a versioned envelope
  `{ "version": 1, "divergences": [...] }` instead of a bare array. The contract is additive-only
  within a major version; a breaking change bumps `version`. Consumers should read
  `.divergences` and tolerate unknown fields (#16).

## [0.2.1] - 2026-06-07

### Documentation
- Precisely scope "cosmetic" (#21): a literal *value* is part of the hashed AST, so editing a
  string literal — including user-facing copy — inside an anchored span trips the gate.
  "Cosmetic" means only whitespace, comments, and consistent renames. Added a FAQ entry and a
  note to anchor the narrowest symbol so unrelated literal edits don't re-open a claim.

## [0.2.0] - 2026-06-07

### Added
- `surf check` scoping: `--files <globs>` evaluates only claims whose anchored file(s) match,
  and an explicit `--base <ref>` diff-scopes the gate to claims whose files changed since the
  merge base. Omitting `--base` keeps a full check (enrichment falls back to `HEAD`).
- `--format json` for `surf lint` and `surf verify`, mirroring `surf check`. `lint` emits
  structured findings; `verify` emits per-anchor outcomes (stamped/followed/unchanged/skipped)
  plus counts.
- Advisory `surf lint` granularity warnings (never blocking): a near-whole-file anchor span, a
  hub with too many anchors, and public functions in an anchored file that no claim covers.
- `surf lint` enforces the `AGENTS.md` pointer block: when `AGENTS.md` carries a
  `<!-- surf:hubs -->` block, it must link the hubs directory (and that directory must exist) so
  agents are pointed at the hubs to search — never duplicating or enumerating them. Opt-in;
  blocks on a missing/dangling pointer.
- `surf suggest <globs>` — scans source for top-level public functions that no hub anchors yet
  and prints a copy-pasteable starter hub. Suggestions only: never writes a file or stamps a hash.
- `surf_core::public_fns` — enumerates a file's top-level public functions (backs the
  under-coverage warning and `surf suggest`).

### Changed
- An explicit `surf check --base <ref>` now **diff-scopes** the gate to claims whose files
  changed, in addition to feeding advisory `old_code`/`magnitude`. Previously `--base` only fed
  enrichment and every claim was still checked. CI that passes `--base` explicitly will now check
  fewer claims; omit `--base` for a full check.

## [0.1.1] - 2026-06-06

### Fixed
- `resolve`: resolve const-bound call-expression functions (TS/JS).
- `verify`: skip unchanged anchors instead of re-stamping, so a no-op verify leaves hub files
  byte-identical.

### Changed
- Releases: dropped Intel macOS (`x86_64-apple-darwin`); prebuilt binaries cover macOS (Apple
  Silicon) and Linux (x86_64). Other targets build from source.

## [0.1.0] - 2026-06-06

Initial release — the MVP gate that surfaces docs↔code divergence.

### Added
- AST-canonical hashing via bundled, version-pinned tree-sitter grammars: quiet on cosmetics
  (formatting, comments, consistent renames), loud on logic. Advisory tree-edit `magnitude`.
- Anchor resolution: the `file > A > B` path grammar with `@N` disambiguation; scope-set
  resolution so `Type > method` resolves uniquely.
- Hub format (markdown frontmatter), `surf.toml` workspace discovery, and per-claim combined
  hashing across multi-site `at:` lists.
- Commands: `surf lint`, `surf check` (the gate), `surf verify` (re-seal, with `--follow` for
  renames), plus `surf init` and `surf new` for authoring ergonomics.
- Language support: TypeScript/TSX, JavaScript/JSX, Rust, Python, and Go.
- Distribution: GitHub Action, pre-commit hook, and `install.sh`; Apache-2.0 license.

[Unreleased]: https://github.com/Connorrmcd6/surface/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/Connorrmcd6/surface/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/Connorrmcd6/surface/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Connorrmcd6/surface/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/Connorrmcd6/surface/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Connorrmcd6/surface/releases/tag/v0.1.0
