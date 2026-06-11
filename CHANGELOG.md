# Changelog

All notable changes to Surface are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `surf suggest --all` additionally proposes the non-callable anchor targets `resolve` already
  accepts but `suggest` previously hid: top-level classes, module-level constants and type
  aliases, and class attributes (Python). The default stays callables-only (functions + methods)
  to avoid over-anchoring fatigue. Closes the gap where those kinds were anchorable by hand yet
  undiscoverable via `suggest` (#52).
- npm distribution: install via `npm install @gradientdev/surface` or run ad-hoc with
  `npx @gradientdev/surface check`. A thin shim resolves the prebuilt binary for the host from
  per-platform `optionalDependencies` (macOS Apple Silicon, Linux x86_64) — no `postinstall`
  download step. Published from CI on each release with provenance attestation (#46).
- Release integrity: each published binary now ships a `<asset>.tar.gz.sha256` checksum, and the
  `curl | sh` install script (and the GitHub Action that wraps it) verifies the download against
  it before installing. A missing checksum or a mismatch aborts the install — fail closed —
  rather than running a corrupted or tampered binary (#39).

### Changed
- `surf stats` is much faster on large histories. It previously recomputed the hub claim set at
  every commit *and* its parent, each spawning `git ls-tree` plus a `git show` per hub — roughly
  `2 × commits × hubs` subprocesses. The claim set is now memoized by commit SHA, with each
  `before` rev canonicalized to its parent's SHA so it reuses the parent commit's already-computed
  state. Metrics are unchanged (#40).

### Fixed
- `surf lint`'s under-coverage nudge is no longer workspace-blind and no longer skips methods.
  Coverage is now computed across the whole workspace, so splitting one file's claims across two
  hubs no longer makes each hub warn about the symbols the other one anchors; and the measured
  public surface now includes methods (`Type > method`), not just top-level functions, matching
  what `suggest` proposes (#54).

## [0.5.0] - 2026-06-09

### Fixed
- `surf check` now fails **closed**. A hub whose frontmatter can't be parsed previously slipped
  through silently (exit 0, unenforced); it now surfaces as an `Unresolvable` divergence that
  blocks the run. A hashing error is likewise reported as `Unresolvable` instead of being counted
  clean (#35).

### Changed
- Hub iteration is unified behind `Workspace::iter_hubs()`, shared by `check`, `lint`, `suggest`,
  and `for`. Each command carries the per-hub parse result explicitly, so the error case can no
  longer be handled inconsistently from one command to the next (#36).
- CI now runs the test suite and dogfood self-check on macOS (`macos-14`) in addition to Linux,
  matching the Apple-Silicon binary that releases ship. Windows is documented as unsupported
  (#37).

## [0.4.0] - 2026-06-09

### Added
- `surf for <path> [symbol]` — reverse lookup: list every hub and claim anchored into a file, so
  you can pull up the documentation governing code *before* you edit it (the inverse of
  `suggest`). Read-only and always exits 0; `--format json` emits a versioned
  `{version, path, matches}` envelope. An optional trailing `symbol` narrows to anchors whose
  first segment matches.
- `surf stats [--since <date>] [--until <date>]` — adoption metrics from git history (advisory,
  never a gate): the **rubber-stamp rate** (re-stamps that changed a claim's stored hash but left
  its prose untouched) and the **in-place update rate** (commits touching an anchored file that
  re-sealed the claim in the same commit). One commit = one PR (merges excluded); heuristics are
  documented in `docs/guides/stats.md`. Errors (non-zero) when git history is unavailable.
- `surf suggest` now proposes **methods**, not just top-level functions: Python class methods and
  Go methods are enumerated as `file > Type > method` anchors (matched by exported Go receiver
  type; Python async/sync mirrors de-duped). On method-oriented codebases this surfaces the bulk
  of the public API that was previously invisible to `suggest`. Rust `impl`/TS class methods
  remain top-level-only.

### Changed
- `surf suggest` distinguishes a glob that matched **no files** from a clean "nothing to suggest":
  it warns on stderr and exits non-zero when *every* glob was empty, so a typo'd path no longer
  reads as success. Coverage is now keyed on the full `file > seg > seg` anchor path, so anchoring
  one method no longer suppresses its siblings.

## [0.3.2] - 2026-06-08

### Documentation
- The docs site (surface.gradientdev.xyz) is now **generated** from this repo on release: each
  `v*` tag regenerates the Starlight site's pages and changelog from `docs/` and `CHANGELOG.md`
  and opens a sync PR, so the site no longer drifts from canonical docs.
- `docs/reference/commands.md` is now governed by `surf check` itself — a hub anchored to the
  clap command/flag surface blocks the gate when the CLI and its reference drift.
- Pinned `Connorrmcd6/surface@vX.Y.Z` Action refs in the README and docs are now derived from
  `Cargo.toml` at release, and the Examples page description is quoted so it parses under the
  site's strict YAML.

## [0.3.1] - 2026-06-08

### Documentation
- Reposition around *documentation governed like code* for fast-moving codebases, led by the
  context-rot story (a context file that's accurate when written and rots as the code moves
  because nobody knows it exists or where to find it). Dropped the older accusatory framing.
- Slim the README to a pitch + compact quickstart that links out; the full reference now lives in
  `docs/`. Restructured `docs/` into a site-ready tree (`getting-started/`, `reference/`,
  `guides/`) with `title`/`description` frontmatter, mirroring the docs site
  (surface.gradientdev.xyz); this repo's `docs/` is the source of truth.
- Bring the docs current with 0.3.0: the versioned `--format json` envelope, the per-claim
  `ignore_literals` option, Python non-callable anchors, file-rename `--follow`, and decorators in
  the hashed span.

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

[Unreleased]: https://github.com/Connorrmcd6/surface/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/Connorrmcd6/surface/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/Connorrmcd6/surface/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/Connorrmcd6/surface/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/Connorrmcd6/surface/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/Connorrmcd6/surface/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/Connorrmcd6/surface/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Connorrmcd6/surface/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/Connorrmcd6/surface/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Connorrmcd6/surface/releases/tag/v0.1.0
