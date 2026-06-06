# Phase 7 — Distribution & CI integration

**Goal:** make the gate actually run in a repo. Ship first the channels repos actually
consume (most *consume the Action*, they don't "install").

**Proposal refs:** §9.1.5 (Action + pre-commit + config discovery), §10 (build once, wrap many; ship order).

**Depends on:** Phase 5.

**Status:** not started

## Steps

1. **Static binary** per `(os, arch)`, built in release CI:
   - start with `aarch64-apple-darwin` (this dev machine);
   - add `x86_64-apple-darwin` and `x86_64-unknown-linux-gnu` / `-musl` (Linux is what CI runs).
2. **GitHub Action wrapper** (`action.yml`) — the primary distribution channel (§10). Runs
   `surf check` on PRs. **Correct checkout: shallow, not `fetch-depth: 0`** (§9.1) — only a
   shallow merge-base fetch if PR-scoping is enabled.
3. **pre-commit hook** definition (`.pre-commit-hooks.yaml`) so repos can run `surf check`/`lint` locally.
4. **`curl | sh` installer** (`install.sh`) — detects `(os, arch)`, downloads the matching release binary.
5. **Defer** npm (shim + per-platform `optionalDependencies`, never a `postinstall`
   downloader), pip (`maturin` wheels), and brew (§10) — don't ship channels nobody uses yet.

## Files touched
- `.github/workflows/release.yml` (build + upload per-target binaries)
- `action.yml`
- `.pre-commit-hooks.yaml`
- `install.sh`

## Verify
- In a throwaway test repo: the Action **blocks** a PR that edits an anchored span, and
  **passes** once `surf verify` is run in the same PR.
- The pre-commit hook fires locally on an anchored-span edit.
- `install.sh` fetches and runs the right binary on `aarch64-apple-darwin`.

## After this: stop and measure (§9.1 / §9.2)
Do not build deferred items (§9.3) until their named trigger fires. Seed 1–2 high-churn,
high-stakes domains (the Rust auth core we dogfood is a natural first hub) and measure over
~6–8 weeks; the `verify`-without-prose-edit (rubber-stamp) rate is the key kill signal.
