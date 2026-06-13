---
name: create-pr
description: Open a pull request for the current work following this repo's conventions — branch off main, run the gates, commit with the Co-Authored-By footer, push, and open a PR with the 🤖 footer and a Closes #N link. Use when the user asks to "open a PR", "make a PR", "ship this", or "raise a PR" for changes in progress or just-made.
---

# create-pr

Take the current change from working tree → open pull request, the way this repo does it.

## Before opening

1. **Never commit to `main`.** If on `main` (or a stale/merged branch), create a fresh branch named for the work: `feat/<n>-slug`, `fix/<n>-slug`, `test/<n>-slug`, `chore/<n>-slug`, `docs/<n>-slug` — match the change type, and include the issue number when there is one. Branch off the latest `main` (`git checkout -b <branch> main`).
2. **Run the gates and quote the result in the PR.** A PR with red gates wastes a review:
   - `cargo fmt --all --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all`
   If `fmt --check` fails, run `cargo fmt --all` and re-verify. Fix clippy/test failures before opening — don't open a PR you know is red.
3. **Stage deliberately.** Add only the files that belong to this change (don't `git add -A` blindly); include `Cargo.lock` when deps changed.

## Commit

- One focused commit (or a small, logical series). Conventional-commit subject: `type(scope): summary (#N)`.
- Body: *why*, not a file list. End every commit message with:
  ```
  Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
  ```

## Open the PR

- `git push -u origin <branch>`, then `gh pr create`.
- PR body: a short summary, what changed and why, and a **Verification** section quoting the gate results you actually ran. Link the issue with `Closes #N`.
- End the PR body with:
  ```
  🤖 Generated with [Claude Code](https://claude.com/claude-code)
  ```

## After opening — do NOT merge

This repo's standing rule (see auto-memory `user-handles-merges-when-watching` and `verify-ci-green-before-merge`):

- **Report the PR as mergeable; let the user merge.** Don't run `gh pr merge` unless explicitly asked for *this* PR, and never retry a merge the user declined.
- If asked to merge, **verify CI is green first** (`gh pr checks` / `gh run list`). `main` has no branch protection, so a merge succeeds even on red/pending — that's exactly why you check by hand. Never merge on red or pending.

## Output

Give the user the PR URL, a one-line summary, and the gate results. State plainly whether it's mergeable.

Then add a short **ELI5** — a few plain-English sentences (or a tiny "moving parts" list) explaining what this PR actually does, in the style of the `eli5` skill: analogy-friendly, jargon-free, no code blocks. It's there so the user has something readable to digest while CI runs. Keep it brief — this is a quick gloss, not the full `/eli5` treatment.
