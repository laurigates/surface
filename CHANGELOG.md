# Changelog

All notable changes to Surface are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `surf check` scoping: `--files <globs>` evaluates only claims whose anchored file(s) match,
  and an explicit `--base <ref>` diff-scopes the gate to claims whose files changed since the
  merge base. Omitting `--base` keeps a full check (enrichment falls back to `HEAD`).
- `--format json` for `surf lint` and `surf verify`, mirroring `surf check`. `lint` emits
  structured findings; `verify` emits per-anchor outcomes (stamped/followed/unchanged/skipped)
  plus counts.
- Advisory `surf lint` granularity warnings (never blocking): a near-whole-file anchor span, a
  hub with too many anchors, and public functions in an anchored file that no claim covers.
- `surf_core::public_fns` — enumerates a file's top-level public functions (backs the
  under-coverage warning).

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

[Unreleased]: https://github.com/Connorrmcd6/surface/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/Connorrmcd6/surface/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Connorrmcd6/surface/releases/tag/v0.1.0
