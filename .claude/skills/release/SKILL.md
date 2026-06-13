---
name: release
description: Cut a Surface release the way this repo does it — prepare the version-bump PR (Cargo version, Cargo.lock, doc pins, CHANGELOG) off a release/X.Y.Z branch, then after it merges, tag vX.Y.Z to trigger the build/npm-publish/docs-sync workflow. Use when the user asks to "cut a release", "release X.Y.Z", "prepare a release", or "tag the release".
---

# release

Ship a versioned release. Two phases: **prepare** (a normal PR) and **tag** (the irreversible, outward-facing publish). Never run the tag phase without explicit confirmation — pushing the tag publishes to npm and GitHub Releases and dispatches a sync to the `surface-site` repo.

## Settle the version first

- SemVer. If the user gave a version (`/release 0.7.0`), use it. Otherwise propose one from the work since the last tag (`git tag --sort=-creatordate | head -1`) and the milestone, and confirm before touching anything.
- The `[workspace.package].version` in `Cargo.toml` is the single source of truth; everything else is derived from it.

## Phase 1 — prepare (a PR)

Branch `release/X.Y.Z` off the latest `main`. Then make exactly these edits:

1. **Cargo.toml** — bump `[workspace.package].version` to `X.Y.Z`.
2. **Cargo.lock** — refresh it: `cargo check` (or `cargo build`) so the `surf-core`/`surf-cli` entries pick up the new version. (Do *not* hand-edit it.)
3. **Doc pins** — run `scripts/bump-docs-version.sh`. It reads the version from Cargo.toml and rewrites every `Connorrmcd6/surface@vX.Y.Z` in `README.md` and `docs/` to match. (It deliberately scopes to README + docs — don't manually touch other files' pins.)
4. **CHANGELOG.md** — Keep a Changelog convention:
   - Insert a new `## [X.Y.Z] - YYYY-MM-DD` header directly **below** `## [Unreleased]` and above the existing entries, so the accumulated Unreleased notes become this version's section and `[Unreleased]` is left empty for the next cycle. Use today's date.
   - Update the compare links at the bottom: repoint `[Unreleased]` to `compare/vX.Y.Z...HEAD`, and add `[X.Y.Z]: .../compare/v<prev>...vX.Y.Z`. Backfill any missing intermediate links while you're there.
   - If the Unreleased section is empty, there's nothing to release — stop and tell the user.
5. **Gates** — `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`. All green before the PR.

Commit subject `chore(release): prepare X.Y.Z`, body summarizing the bump (version, doc re-pins, changelog cut), with the `Co-Authored-By` footer. Open the PR via the **`create-pr`** conventions (push, 🤖 footer, ELI5). **Report it mergeable; let the user merge** — don't merge or tag yourself here.

## Phase 2 — tag (publish — confirm first)

Only after the prepare PR is **merged to main**. This is the step that goes out to the world, so:

1. `git checkout main && git pull` — confirm the merge landed and `Cargo.toml` reads `X.Y.Z`.
2. **Verify CI is green on main** (`gh run list --branch main` / `gh pr checks`). `main` has no branch protection; never tag on red or pending.
3. **Confirm with the user** that you're about to push `vX.Y.Z` — name what it triggers (binaries for 3 targets, npm publish under `@gradient-tools/*`, docs sync to surface-site). Wait for an explicit yes.
4. Tag the merge commit and push:
   ```
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
   The tag must equal the Cargo.toml version. Pushing `v*` is what fires `.github/workflows/release.yml`.

## After the tag

- Watch the release run (`gh run list --workflow release.yml` / `gh run watch`).
- **npm visibility is the known-flaky part** (#80): the registry has silently dropped freshly published versions even on green publishes. The workflow now polls each `@gradient-tools/*` package until it's fetchable and fails if any never appears — so let that job finish rather than trusting an early "publish succeeded." If it fails there, the binaries/GitHub release are fine; it's the npm publish that needs re-running.
- Report: tag pushed, run URL, and the publish outcome once the workflow settles.
