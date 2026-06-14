# Surface — security & correctness evaluation

**Scope:** adversarial review of Surface (a Rust doc↔code drift gate), verifying claims by
reading code and running it rather than trusting comments. Conducted on
`laurigates/surface` @ `main` (`7edfeff`), workspace version `0.6.2`.
**Date:** 2026-06-14.

Areas, in priority order: (1) release-pipeline supply chain, (2) hash/normalization
determinism, (3) gate-bypass safety, (4) anchor correctness across languages, (5) performance.

> An upstream PR pinning `install.sh` to the action ref is already in flight and is **not**
> revisited here; this report builds on it.

---

## 0. Baseline

All four gates were run first to establish a clean starting point.

| Command | Result |
|---|---|
| `cargo build --locked` | ✅ exit 0 |
| `cargo test --locked` | ✅ **179 passed, 0 failed** — *after* neutralizing the sandbox (see note) |
| `cargo clippy --locked --all-targets` | ✅ exit 0 (CI runs `-D warnings`) |
| `cargo audit` | ✅ **0 advisories**, 109 crate dependencies |

**Sandbox note (not a product defect).** In this evaluation environment a forced
commit-signing hook returns HTTP 400, so the 11 tests that `git commit` into a tempdir
(`check::tests::*`, `stats::tests::*`) fail with `signing failed`. Pointing
`GIT_CONFIG_GLOBAL` at a config with `commit.gpgsign = false` makes all 179 tests pass. The
failures are environmental; the suite is green on normal CI (and on GitHub's runners, which
don't force signing). Minor hardening idea for `CONTRIBUTING`: the git-backed tests assume
`git commit` succeeds; they could set `-c commit.gpgsign=false` in the `git()` helper so they
pass in signing-enforced environments too. *(LOW — see §6.)*

---

## Findings by severity

| # | Severity | Area | Finding | Deliverable |
|---|---|---|---|---|
| F1 | **HIGH** | Gate bypass | `check --base` silently passes real drift when the workspace is a repo subdirectory | **PR #2** (fix + regression test) |
| F2 | **MEDIUM** | Supply chain | Third-party Actions pinned by mutable tags in `contents:write` / `id-token:write` jobs | **PR #3** (SHA pins) |
| F3 | **MEDIUM** | Determinism | No golden test pins the canonical hash; a grammar bump can silently re-hash every consumer | **PR #4** (golden test) |
| F4 | **MEDIUM** | Supply chain | Release tarballs (`install.sh`'s input) have only a self-attested SHA-256, no verifiable provenance | Ready patch (§1.2) — *propose* |
| F5 | LOW | Supply chain | `notify-site` cross-repo token scope is unverifiable from the repo; document the minimum | §1.3 |
| F6 | LOW | Supply chain | npm post-publish visibility poll (#80) can still under-wait registry propagation | §1.4 |
| F7 | LOW | Anchor correctness | Positional `@N` anchors silently re-point when same-named symbols are reordered/deleted | §4.3 |
| F8 | LOW | Tests | Git-backed tests assume commit signing succeeds | §0 note |

Everything below is reproducible; commands and outputs are included.

---

## 1. Release-pipeline supply chain (`.github/workflows/release.yml`)

The release job runs on a `v*` tag push with `contents: write`, builds three target triples,
uploads `surf-<triple>.tar.gz` + a `.sha256`, then `npm-publish` (`id-token: write`) and
`notify-site` (cross-repo PAT) fan out. `install.sh` and `action.yml` pull those tarballs onto
every consumer's machine and CI runner — so this job *is* the trust root.

### 1.1 Mutable Action tags in privileged jobs — **F2 (MEDIUM) → PR #3**

Every third-party Action was referenced by a mutable major tag:

| Action | Where | Privilege in that job |
|---|---|---|
| `actions/checkout@v6` | ci (×3), release (×3) | — |
| `softprops/action-gh-release@v3` | release `build` | **`contents: write`** (writes release assets) |
| `actions/setup-node@v6` | release `npm-publish` | **`id-token: write`** (npm provenance OIDC) |
| `peter-evans/repository-dispatch@v4` | release `notify-site` | cross-repo PAT |
| `rustsec/audit-check@v2` | ci `audit` | `GITHUB_TOKEN` |

A tag is mutable: its owner — or anyone who compromises that account — can re-point it at
arbitrary code that then runs with the privileges above, tampering with the very binaries
`install.sh` verifies against a hash *the same job produced*. **PR #3** pins each to a
full-length commit SHA with a version comment (the Dependabot/Renovate-maintained format):

```
actions/checkout@df4cb1c069e1874edd31b4311f1884172cec0e10 # v6
actions/setup-node@48b55a011bda9f5d6aeb4c2d9c7362e8dae4041e # v6
softprops/action-gh-release@b4309332981a82ec1c5618f44dd2e27cc8bfbfda # v3
peter-evans/repository-dispatch@28959ce8df70de7be546dd1250a005dd32156697 # v4
rustsec/audit-check@69366f33c96575abad1ee0dba8212993eecbe998 # v2.0.0
```

SHAs resolved via `git ls-remote` against each upstream repo. **`dependabot.yml` already
covers the `github-actions` ecosystem** (verified — it has a `github-actions` / `directory: /`
update block), so the pins stay current.

### 1.2 Tarballs have a self-attested hash, not provenance — **F4 (MEDIUM), propose**

The integrity story for the install path is: the `build` job runs `sha256sum` on the tarball
it just built and uploads the digest beside it; `install.sh` re-downloads both and compares.
That defends against a corrupted/partial download or an asset swapped *after* publication —
but the hash is **self-attested by the same job that built the artifact**. Anyone who can run
that job (a compromised pinned-away Action, a malicious tag build, a leaked token) produces a
matching hash for a malicious binary. The hash proves *integrity*, not *provenance*.

`actions/attest-build-provenance` closes that gap: it signs a SLSA provenance statement via
Sigstore tying each tarball to the workflow, repo, commit, and runner, verifiable offline with
`gh attestation verify`. Ready-to-apply patch for the `build` job (kept out of PR #3 to stay
single-purpose):

```yaml
# release.yml, job: build
    permissions:
      contents: write          # upload release assets
      id-token: write          # Sigstore OIDC for provenance
      attestations: write      # write the attestation
    steps:
      # ... existing checkout / build / package / checksum ...
      - name: Attest build provenance
        uses: actions/attest-build-provenance@<pin-to-SHA>   # v3
        with:
          subject-path: |
            surf-${{ matrix.target }}.tar.gz
      # ... existing "Upload to release" ...
```

Consumers can then optionally `gh attestation verify surf-<triple>.tar.gz --repo Connorrmcd6/surface`
in CI. `install.sh` itself can't verify without `gh`, so keep the SHA-256 as the baseline and
treat provenance as the stronger, opt-in signal — a doc note in `action.yml` pointing CI users
at `gh attestation verify` is the lightweight win. *This belongs upstream too.*

### 1.3 `notify-site` cross-repo token — **F5 (LOW)**

`notify-site` dispatches to `Connorrmcd6/surface-site` with `secrets.SURFACE_SITE_SYNC_TOKEN`.
The token's scope can't be audited from this repo, but the job needs **only** `repository_dispatch`
on the single target repo. Recommendations: use a fine-grained PAT (or a GitHub App
installation token) scoped to `surface-site` with just *Contents: read* + *Metadata* and the
dispatch permission; never a classic `repo`-scoped PAT. The `notify-site` job also has no
explicit `permissions:` block, so it inherits the workflow default `contents: write` it doesn't
use — add `permissions: {}` to that job to drop ambient `GITHUB_TOKEN` scope (defense in depth;
the dispatch uses the PAT, not `GITHUB_TOKEN`).

### 1.4 npm post-publish visibility poll (#80) — **F6 (LOW)**

The `Verify versions are visible` step (added after v0.6.0/v0.6.1 published "successfully" yet
left zero fetchable versions) polls `npm view "$pkg@$version" version` up to 10× with 30s
sleeps = **5 min max** per package, failing the release if a version never appears. This is a
solid guard. Two robustness notes:

- `npm view` hits a CDN-cached registry read; a *miss* can be cached too, and propagation has
  occasionally exceeded 5 min during npm incidents. Consider widening to ~15 attempts and/or
  `npm view --registry=https://registry.npmjs.org/` to bypass any local mirror, and a
  `--cache /tmp/...` throwaway to avoid a stale local cache hit.
- On the first success the loop `continue 2`s without confirming the *shim's*
  `optionalDependencies` resolve to the just-published platform packages. A cheap addition:
  after all four are visible, `npm view @gradient-tools/surface@$version optionalDependencies`
  and assert each pin equals `$version`.

Neither is a correctness bug; both reduce flaky/empty releases.

---

## 2. Hash / normalization determinism (`surf-core`)

**Question:** is the stored anchor hash stable across the three target triples, a rust-version
bump, and tree-sitter grammar bumps?

**How the hash is built** (`surf-core/src/hash.rs`): parse with tree-sitter → walk the symbol's
subtree → drop comments → alpha-rename identifiers to first-occurrence indices (`#0`, `#1`, …)
→ keep operators/keywords/punctuation/literal *values* verbatim → SHA-256 the NUL-separated
token stream → first 12 hex chars. Verified end-to-end (computed via the public API):

```
rust  `add`         f1075e760a17
ts    `Svc > rotate`afa4514b5c89
tsx   `App`         97e0de58725d
py    `add`         879b76118966
go    `Add`         942af2641116
```

**Determinism by construction — verified.** The output depends only on: (a) the token *kinds*
and *text* tree-sitter yields, (b) source-order traversal, (c) SHA-256.
- No floats, no pointer-width-dependent values, no time/random inputs.
- The identifier→index map is a `HashMap`, but indices are assigned by **insertion order**
  (`idents.entry(text).or_insert(idents.len())`) during an ordered DFS, and only the indices —
  never the map's iteration order — reach the output. So `HashMap`'s randomized iteration order
  is irrelevant; the hash is independent of std hasher seed and of platform.
- ⇒ **Stable across the three target triples and across rust-version bumps.** The CI matrix
  (`ubuntu-latest` + `macos-14`) already exercises Linux and Darwin.

**The real risk is grammar versions — and it is unguarded.** Grammars are caret-pinned in
`surf-core/Cargo.toml` (`tree-sitter-rust = "0.24.2"` ⇒ `^0.24.2`, `tree-sitter = "0.26.9"`,
etc.) and frozen only by `Cargo.lock`. `--locked` keeps any *single* release reproducible, but
Dependabot's `cargo-minor-patch` group opens PRs that bump the lock. tree-sitter grammars *do*
rename node kinds and reshape trees across releases. Because the hash is built from node-kind
strings, such a bump silently changes the canonical token stream for unchanged code → every
stored hash in every downstream repo flips to false `DIVERGED` (or, worse, two distinct spans
begin to collide). **There was no test asserting any hash value**, so CI would stay green
through such a change.

**F3 (MEDIUM) → PR #4** adds `surf-core/tests/golden_hash.rs`: exact goldens per language plus
invariants (reformatting + consistent rename keep the hash; operator flip + swapped operands
move it, and `b - a ≠ a - b`). A grammar/normalization drift now fails loudly, on both CI
platforms — exactly the "test that would catch cross-platform drift" requested. Verified
catching: temporarily perturbing the canonicalizer flips the goldens; the suite passes on the
pinned grammars.

> **Propose upstream:** the same `golden_hash.rs` applies verbatim to `Connorrmcd6/surface`.

---

## 3. Gate-bypass safety (`surf-cli` check / scope logic)

**Question:** can a crafted `--base` or `--files` silently scope the gate to zero anchored
files and still exit 0? I traced every fail-closed path and tested the boundaries.

### 3.1 What is already correct (verified)

- **`--files` zero-match (#78).** `run()` computes
  `all_empty = !files.is_empty() && unmatched_globs.len() == files.len()` and forces
  `ExitCode::FAILURE`. A wholly-typo'd `--files` cannot read clean; a *partially* matching set
  still succeeds. Confirmed by `zero_match_files_glob_is_reported_and_fails_alone` and
  `partially_matching_files_globs_still_succeed`, and re-ran by hand.
- **Invalid glob syntax (#38)** hard-errors (`Scope::build` returns `Err`), never silently
  widens/narrows scope. Verified (`invalid_files_glob_syntax_errors`).
- **Unparseable frontmatter fails closed (#35).** A malformed hub becomes an `Unresolvable`
  divergence → non-zero exit, not a skip. Verified (`malformed_hub_blocks_check`).
- **Bad `--base` ref / non-repo / shallow clone.** `git::changed_files` returns `None` on any
  git failure, and `Scope` then falls back to a **full** check rather than an empty one — the
  safe direction. Traced through `merge-base` (filtered on `.status.success()`, else falls back
  to the raw ref) and `diff` (`output.status.success().then(...)` ⇒ `None` on a bad ref). A
  non-repo invocation likewise degrades to a full, git-free check.

### 3.2 F1 (HIGH) — `--base` bypass when the workspace is a repo subdirectory → PR #2

`git::changed_files` ran `git diff --name-only <base>`, whose paths are **repo-root-relative**.
But `Anchor.file` is **workspace-root-relative** (relative to the dir holding `surf.toml`).
`Workspace::discover` walks *up* for `surf.toml`, so the workspace root is frequently a
**subdirectory** of the git repo (monorepo, nested package, `docs/` site, etc.). In that
layout the changed-file set (`proj/src/x.rs`) never intersects an anchor file (`src/x.rs`), so
`Scope::includes` filters out **every** claim → zero claims checked → **clean exit 0**, despite
real drift. This is the exact diff-scoped mode `action.yml` *recommends* for PR gating, so the
bypass sits on the CI happy path.

**Reproduced from scratch** (`/tmp` repro, full transcript in the PR): with the workspace in
`proj/`, the anchored span committed as `a + b` and the working tree changed to `a - b`:

```
surf check              → exit 1  (DIVERGED)        ✅
surf check --base HEAD  → exit 0  ("all spans match") ❌  BYPASS
```

Same setup with the workspace *at* the repo root: both exit 1. That asymmetry is why the
existing `base_scope_limits_to_changed` test (repo root == workspace root) never caught it.
Confirmed deterministic (5/5 runs exit 0) and confirmed the mechanism:
`git -C proj diff --name-only HEAD` → `proj/src/m.rs` while the anchor is `src/m.rs`.

**Fix (PR #2):** add `--relative` so git emits workspace-root-relative paths
(`git -C proj diff --name-only --relative HEAD` → `src/m.rs`). No-op when the workspace is the
repo root; additionally drops out-of-workspace changes that can never be anchored. One line in
`surf-cli/src/git.rs`, plus `base_scope_works_when_workspace_is_a_repo_subdir`, which inits the
repo at the *parent* of the workspace and fails without the fix (asserts 0 vs 1) and passes
with it.

> **Propose upstream:** identical bug and fix in `Connorrmcd6/surface`.

### 3.3 Residual, *intended* behavior (not a finding)

A *valid* `--base` whose diff genuinely touches no anchored file yields zero in-scope claims
and exit 0 — that is correct diff-scoping (nothing changed ⇒ nothing to gate). The danger is
only the silent path-mismatch in §3.2, now fixed.

---

## 4. Anchor correctness across languages

`parse_anchor` grammar: `path > A > B > C`, optional `@N` positional suffix per segment.
Resolution (`resolve.rs`) walks segment-by-segment over scope *sets* (so `Type` + its `impl`
both descend), with a Go-specific flat resolver and Python `@overload` grouping (#82).

### 4.1 Cosmetic-vs-logic boundary — probed, holds

Verified directly against the engine (see §2 goldens for the values):

| Edit | Expectation | Result |
|---|---|---|
| add a comment / reflow whitespace | hash unchanged | ✅ `f1075e760a17` == canonical |
| consistent local rename `a,b → x,y` | hash unchanged | ✅ == canonical |
| operator flip `a + b → a - b` | hash changes | ✅ `a65eded7c38e` |
| swap operands `a - b → b - a` | hash changes, ≠ flip | ✅ `f074604a5b1f` |
| Python decorator `@cache → @lru_cache` | hash changes | ✅ decorator name kept verbatim (`emit` decorator path) |
| reorder two independent statements | hash changes | ✅ the token stream is ordered, so any reordering is caught (loud — a deliberate design choice; reordering is treated as a change, never silently ignored) |

The alpha-rename is *consistent*-rename invariant but order-sensitive: swapping two names
without swapping their uses changes the stream. No false-negative found on the logic side; the
"quiet on cosmetics, loud on logic" contract holds across all five languages tested.

### 4.2 `parse_anchor` robustness — probed

- Multiple `@` (`a@2@3`) → `split_once('@')` keeps the first, `2@3` fails `usize::parse` →
  `BadIndex("2@3")` (loud). `@0`, `@`, `@x`, empty segment, missing symbol, empty file — all
  rejected with distinct errors (matches the in-file unit tests; re-read and confirmed).
- These are covered by the existing `anchor::tests`. **Recommended addition (propose upstream):**
  a `proptest`/fuzz harness over `parse_anchor` asserting it never panics and round-trips
  `Display`→`parse` for well-formed anchors, and over the `@N` selector asserting
  `resolve(@k)` ⊆ `resolve(no-index)` for every `k`. Not filed as a PR because it pulls in a new
  dev-dependency (`proptest`) — a maintainer call; sketch:

  ```rust
  // surf-core/tests/anchor_fuzz.rs  (needs `proptest` dev-dep)
  proptest! {
    #[test]
    fn parse_never_panics(s in ".{0,64}") { let _ = surf_core::parse_anchor(&s); }
    #[test]
    fn index_selects_subset(seg in "[A-Za-z_][A-Za-z0-9_]{0,8}", k in 1usize..6) {
        // build a source with N same-named fns; assert resolve(@k) is one of resolve(no @)
    }
  }
  ```

### 4.3 F7 (LOW) — positional `@N` fragility

`@N` is 1-based positional over same-named symbols in document order. If a sibling with the
same name is **added/removed/reordered** above the anchored one, `@N` silently re-points to a
*different* definition. Its hash will then usually differ → a (correct-by-accident) `DIVERGED`,
but if the new Nth symbol happens to be token-identical, the gate passes while pointing at the
wrong code. This is inherent to positional anchoring and already hedged by `resolve`'s
"must be unique" default (no `@N` ⇒ ambiguity is an error). Recommendation: keep `@N` a
last-resort escape hatch, and have `surf lint`/`suggest` warn when an `@N` anchor exists
alongside a now-unique name (the `@N` is stale and should be dropped). Documentation/lint
nicety, not a gate hole.

---

## 5. Performance

No `hyperfine` in the environment; timed the release binary over a generated **2,000-symbol
polyglot tree** (500 files each Rust/TS/Python/Go, 10 hubs × 200 anchors, 8.1 MB).

| Command | Wall time (best of 3) | Notes |
|---|---|---|
| `surf check` (2,000 anchors: resolve + hash every span) | **~0.10 s** | full gate |
| `surf lint` | **~0.09 s** | resolve + frontmatter validation |
| `surf check --files 'src/rs/*.rs'` (scoped) | **~0.03 s** | glob-narrowed |

≈ 20k anchors/s, single-threaded (no `rayon`; cost is process start + I/O + tree-sitter parse).
Comfortably inside pre-commit/CI budgets — a realistic repo (hundreds of anchors) is single-digit
**milliseconds** of engine work after startup. No optimization needed; if a 50k-anchor monorepo
ever appears (~2.5 s extrapolated), parallelizing the per-hub loop is the obvious lever.

---

## 6. Deliverables & upstream proposals

**Draft PRs opened against `laurigates/surface`:**

| PR | Severity | Title |
|---|---|---|
| #2 | HIGH | `fix(check): honor --base scope when workspace is a repo subdirectory` |
| #3 | MEDIUM | `ci: pin third-party Actions to commit SHAs` |
| #4 | MEDIUM | `test(core): golden hash determinism guard across languages and versions` |

**Propose-upstream items** (apply verbatim to `Connorrmcd6/surface`; can only push to the fork):

1. **F1 / F3 / F2** — PRs #2, #3, #4 above are all upstream-applicable as-is.
2. **F4 — build provenance** — ready patch in §1.2 (`actions/attest-build-provenance` on the
   `build` job + an `action.yml` doc note on `gh attestation verify`).
3. **F5 — `notify-site` token** — fine-grained, single-repo dispatch token; add `permissions: {}`
   to the job (§1.3).
4. **F6 — npm poll** — widen attempts / bypass cache / assert shim `optionalDependencies` (§1.4).
5. **§4.2 — `proptest` fuzz harness** for `parse_anchor` and `@N` (new dev-dep; maintainer call).
6. **F8 — test hygiene** — set `commit.gpgsign=false` in the `git()` test helpers so the
   git-backed suite passes under signing-enforced environments (§0).

**Verification summary:** every HIGH/MEDIUM finding is backed by a from-scratch reproduction or
a failing-without-fix test, not by reading comments. Baseline gates pass; the three PRs keep
`cargo clippy -D warnings` clean and the full suite green.
