<!--
NOTE TO THE BUILDING AGENT
==========================
This is the positioning / selling layer of the README, written as drop-in prose.
Sections marked [FILL] are yours — install, quickstart, config, command reference, etc.
Two things to preserve as you fill in the technical detail:

1. Keep the honesty. The "What Surface does NOT do" section is a feature, not a
   disclaimer. Do not soften it or move it to the bottom. Devs trust tools that
   state their limits.
2. Do NOT lead with the security example anywhere. Surface cannot catch
   action-at-a-distance breaches (a change to code no hub anchored). Leading with
   "catches the auth bypass" sets up "it missed the breach" blowback. Security is a
   nice side effect, never the headline.

Voice target: ruff / esbuild / tailwind READMEs. Confident, concrete, low on
adjectives, opinionated. Short lines. No "revolutionary," no "seamless."
-->

# Surface

**Your test suite is green. Your documentation is lying. Surface catches the second one.**

Tests verify that your code is correct. Surface verifies that your *description of the code* is correct. When the code a doc describes changes out from under it, Surface fails the build — the same way a broken test does.

Part of [**Gradient**](https://gradientdev.xyz). Deterministic. No model, no network, no API key in the core.

---

## The problem

Documentation doesn't break loudly. It rots silently.

Someone refactors a function. The behavior changes on purpose, the tests get updated to match, CI goes green, the PR merges. Everything is correct. But the paragraph in your architecture doc that *described* that function — nobody touched it. It now says something that is no longer true.

Nothing failed. Nothing fired. The code works perfectly. The only thing that broke is the explanation a human (or an agent) will read six months from now to understand the system — and they will trust it, and they will reason from a false premise, and no one will ever know why the decision went sideways.

A codebase can be fully green on tests and full of confident, well-written, completely false documentation. That second failure is the one that quietly poisons everyone who reads it, and you have no tool that catches it.

That's the gap Surface fills.

## What Surface does that tests don't

It's worth being precise, because "documentation linter" undersells it.

A test asserts that **behavior** matches an expectation written in code. Surface asserts that **prose** still matches the code it describes. Those are different expectations, and they drift apart at exactly the moment someone updates one and forgets the other.

|                    | docs accurate          | docs stale                          |
| ------------------ | ---------------------- | ----------------------------------- |
| **code correct**   | fine                   | **← nothing else catches this**     |
| **code broken**    | your tests catch this  | both might fire                     |

The bottom-left cell is what tests are for. The top-right cell — code that works fine but no longer does what your docs claim — has no owner. You can't write a unit test for "the README still describes this accurately," because the thing that drifted is human-language understanding, and tests don't speak that language.

Surface owns that cell.

## How it works, in one breath

You anchor a sentence to the code it's about:

```yaml
# auth/_hub.md  (a "hub" — frontmatter + prose, lives next to the code it describes)
anchors:
  - claim: "refresh rotation is single-use; reuse triggers global logout"
    at: "src/auth/refresh.ts > rotateRefreshToken"
    hash: 9b1c33a
```

`surf check` runs in CI. For each anchor it finds the function, reduces it to pure logic (ignoring formatting, comments, and renames — see below), fingerprints that, and compares the fingerprint to the one stored from the last time a human confirmed the sentence was true.

- **Fingerprint matches** → the logic didn't meaningfully change → silent pass.
- **Fingerprint differs** → the code *diverged* from its description → **block the merge** and hand back a precise report: which hub, which claim, old code vs. new code.

Think of it as a **tamper-evident seal on the logic of exactly the code your docs claim to describe.**

The clever part is what counts as "meaningfully change." Surface compares the *canonical syntax tree*, not the text. So:

- reformat the function, add a comment, rename a local variable → **invisible.** No false alarm.
- flip a `+` to a `-`, change a `<` to `<=`, drop an `await` → **fires.** The logic actually moved.

Quiet on cosmetics, loud on logic. That's the entire reason this beats grepping diffs or eyeballing it in review.

## What Surface does NOT do

Read this part. It's the difference between a tool you trust and one that burns you.

- **It does not tell you your docs are *true*.** It tells you the code they point at *changed*, so a human should re-read the prose. A green check means "nothing drifted since the last sign-off," not "everything is correct." That's a deliberately weaker promise than a passing test, because meaning isn't mechanically decidable.
- **It only watches what you anchored.** If a change in a file no hub points at quietly invalidates a documented invariant, Surface will not see it. Catching that is security review and taint analysis — a different discipline. Surface guards the spans you chose to describe, nothing more.
- **It is not a retrieval system.** It doesn't search, embed, or serve context. There are good tools for that. Surface optimizes a different thing: *trust* in what you retrieve.

If you want the fuzzy "is this claim still true" judgment, that lives in an **optional** reviewer plugin that reads Surface's JSON output. The core never depends on it. Pull every plugin out and the gate blocks and passes exactly the same.

## Is Surface for you?

Honestly? Maybe not. Roughly, it earns its keep when:

> **codebase complexity × change velocity × (humans + agents) reading it**

is high. A small, slow, simple codebase doesn't need this — your team can just read the code, and two well-kept markdown files beat the whole apparatus. Adopt Surface there and it's pure ceremony. Use it where rebuilding the mental model from source is genuinely expensive and the code moves fast enough to drift.

One thing that pushes the math toward "yes": **AI agents.** A human onboards onto a domain once and amortizes the cost over months. An agent re-onboards every single session and amortizes nothing — it's a new hire on its first day, every day, and it pays the full cost of wrong context on every invocation. A bigger context window doesn't fix this; it lets an agent read every line and still confidently derive a *wrong* model, because it can't tell a deliberate invariant from incidental code. If your team runs agents hard, your effective "readers" count is enormous, and accurate, governed context stops being optional.

But note: Surface is justified *without* a single agent in the loop. "Your architectural invariants are now governed like code" pays off for human onboarding, review, and incident response on its own. Agents are a multiplier, not the foundation. That's by design — it's why this survives whatever happens to the AI hype cycle.

---

## Install

Surface is one static binary. Today it builds from source; the consume-the-gate channels
land with the first public release.

**From source (works today)** — requires [Rust](https://rustup.rs):

```sh
git clone https://github.com/Connorrmcd6/surface
cd surface
cargo install --path surf-cli      # puts `surf` on your PATH (~/.cargo/bin)
# or: cargo build --release        # binary at target/release/surf
```

**Consume the gate** — most repos never install a binary; they run the Action or the
pre-commit hook.

GitHub Action — `.github/workflows/surface.yml`:

```yaml
name: Surface
on: pull_request
jobs:
  surface:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4   # plain checkout — do NOT set fetch-depth: 0
      - uses: Connorrmcd6/surface@v0.1.0
```

pre-commit — `.pre-commit-config.yaml`:

```yaml
- repo: https://github.com/Connorrmcd6/surface
  rev: v0.1.0
  hooks:
    - id: surf-check
```

curl:

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/Connorrmcd6/surface/main/install.sh | sh
```

> Prebuilt binaries are published for **macOS (Apple Silicon)** and **Linux (x86_64)**. On
> Intel macOS, Windows, or other architectures, build from source with `cargo install`.

## Quickstart

Set up the workspace, then scaffold a hub — a markdown file whose frontmatter anchors
sentences to code:

```sh
surf init              # writes surf.toml + creates hubs/
surf new auth          # creates hubs/auth.md
```

Edit it: write a claim and point `at:` at the symbol it describes.

```yaml
---
summary: How auth refresh rotation works.
anchors:
  - claim: refresh rotation is single-use; reuse triggers global logout
    at: src/auth/refresh.ts > rotateRefreshToken
---

# Auth

Prose a human (or agent) reads to understand this domain.
```

Then drive the loop:

```sh
surf lint     # does every anchor resolve to exactly one symbol?
surf check    # the gate — a brand-new claim is "unverified" until you seal it
```
```
UNVERIFIED  hubs/auth.md :: src/auth/refresh.ts > rotateRefreshToken
    run `surf verify`
```

You've read the prose and confirmed it's true, so seal it — this writes the hash back into
the frontmatter (`verify` only touches that one line):

```sh
surf verify
surf check    # surf check: all anchored spans match their stored hashes.
```

Now change the *logic* of `rotateRefreshToken` and run the gate again:

```
$ surf check
DIVERGED  hubs/auth.md :: src/auth/refresh.ts > rotateRefreshToken
    stored 9b1c33ade8f1 → now 4d5e6f2a0b7c  (magnitude: Small)
    claim: refresh rotation is single-use; reuse triggers global logout
surf check: 1 divergence(s).
```

The merge is blocked (non-zero exit) until someone re-reads the sentence. If it still holds,
`surf verify` re-seals; if it's now false, fix the prose first. Reformatting, comments, or
renaming a local variable do **not** trip it — only logic does.

Machine-readable output for tooling and the optional reviewer plugin:

```sh
surf check --format json
```
```json
[
  {
    "hub": "hubs/auth.md",
    "claim": "refresh rotation is single-use; reuse triggers global logout",
    "at": "src/auth/refresh.ts > rotateRefreshToken",
    "kind": "changed",
    "old_hash": "9b1c33ade8f1",
    "new_hash": "4d5e6f2a0b7c",
    "new_code": "function rotateRefreshToken(...) { ... }",
    "prose": "refresh rotation is single-use; reuse triggers global logout",
    "magnitude": "small"
  }
]
```

## Commands

- `surf init` — bootstrap a workspace: write `surf.toml` and create the hubs directory (idempotent).
- `surf new <name>` — scaffold a new empty hub under your hubs directory.
- `surf lint [--format human|json]` — validate frontmatter and that every `at:` resolves to exactly one symbol. Warns on a renamed symbol (suggests `verify --follow`); blocks on ambiguous or vanished anchors. Also emits advisory granularity warnings (never blocking, §8): a near-whole-file anchor span, a hub with too many anchors, and public functions in an anchored file that no claim covers.
- `surf check [--format human|json] [--base <ref>] [--files <globs>]` — the gate. AST-canonical-hash each anchored span and compare to the stored hash; non-zero exit on any divergence. By default every claim is checked. `--base <ref>` scopes to claims whose anchored files changed since the merge base **and** recovers the advisory `old_code` / `magnitude` fields from that ref (omit it for a full check with enrichment against `HEAD`). `--files <globs>` scopes to claims whose anchored file(s) match a comma-separated glob (e.g. `surf-core/**`).
- `surf verify [<at>] [--follow] [--format human|json]` — re-seal after you've confirmed the prose still holds; writes the hash into the frontmatter. `<at>` limits to one anchor; `--follow` re-points a renamed single-symbol anchor and re-hashes in one step.

## Documentation

Full docs live in [`docs/`](docs/index.md):

- [Authoring hubs](docs/guides/authoring-hubs.md) — writing claims and anchors, choosing granularity, the verify loop.
- [CI integration](docs/guides/ci-integration.md) — the Action, the pre-commit hook, and scoping the gate to a PR.
- [Examples](docs/examples.md) — a minimal worked hub in each supported language.

Release history is in [`CHANGELOG.md`](CHANGELOG.md). AI agents working in this repo: see [`AGENTS.md`](AGENTS.md).

## Configuration

A `surf.toml` at the repo root marks the workspace — `surf` walks up from the current
directory to find it, like `git` or `ruff` — and globs your hubs:

```toml
hubs = ["hubs/*.md"]
```

Point the glob wherever your hubs live: keep them central (`hubs/*.md`) or co-locate them
with code (e.g. `["**/_hub.md"]`).

**Languages:** TypeScript (`.ts`, `.tsx`, `.mts`, `.cts`), JavaScript/JSX (`.js`, `.jsx`,
`.mjs`, `.cjs`), Rust (`.rs`), Python (`.py`, `.pyi`), and Go (`.go`). Grammars are compiled
into the binary and version-pinned, so a hash computed on your laptop and in CI always agree.

**CI:** the gate hashes your working tree and compares it to the hash committed in the
frontmatter. It needs the checkout, **not** the history — do **not** set `fetch-depth: 0`.
(The advisory `old_code` / `magnitude` use a single `git show` of the base ref; with no git
available the verdict is unchanged, those fields are just omitted.)

## How the gate works (technical)

1. **Locate.** tree-sitter parses the file and resolves the `at:` path (a qualified
   `file > A > B` path, with `@N` for genuine name collisions) to the exact node span. A
   scope is treated as a *set* of nodes, so a type and its `impl`/methods — which share a
   name — disambiguate by path: `Type` alone is ambiguous, `Type > method` is unique.
2. **Canonicalize.** Walk that span's syntax tree into a token stream. Whitespace and
   comments aren't in the tree, so they drop out for free; identifiers are alpha-renamed to
   positional placeholders (a *consistent* rename yields the same tokens, swapping two names
   does not); operators, keywords, and literal *values* are kept verbatim.
3. **Hash.** SHA-256 of that stream, truncated to 12 hex. A list `at:` combines its sites
   into one hash, so the claim is stale if *any* listed span changes.
4. **Compare** against the hash stored in the frontmatter (written by `surf verify`). Equal
   → pass; different → block.

Quiet on cosmetics, loud on logic — and **reproducible**, because the parser ships *inside*
the binary and is version-pinned. There is no separate formatter or language server in CI to
skew the result.

The `--format json` report is the seam every optional layer reads. Per diverged claim:
`hub`, `claim`, `at`, `kind` (`changed` | `unverified` | `unresolvable`), `old_hash`,
`new_hash`, `old_code`, `new_code`, `prose`, `magnitude`. `magnitude` (`small`/`medium`/
`large`) is advisory triage only — it helps a human decide which blocked claim to read
first, and it **never** affects pass/fail.

## FAQ

**Isn't this just tests?** No — see the 2×2 above. A test asserts that *behavior* matches an
expectation written in code; Surface asserts that *prose* still matches the code it
describes. Different expectation, different failure mode, and they drift apart exactly when
someone updates one and forgets the other.

**Why not just put doc comments next to the code?** Co-located comments still rot silently —
nothing gates them. Surface is the gate; your prose can live wherever you like, but the seal
is what's enforced in CI.

**Does it slow CI down?** No. It parses and hashes a handful of spans — I/O-bound, not
compute-bound. No model, no network, no API key.

**What languages?** TypeScript, JavaScript/JSX, Rust, Python, and Go today, via bundled
tree-sitter grammars. More are a build-time addition to the binary, never a runtime dependency.

**What does a green check actually promise?** That nothing you anchored has changed since it
was last verified — *not* that your docs are true, and nothing at all about code you didn't
anchor. See "What Surface does NOT do."

---

<sub>Surface is part of **Gradient**. The naming isn't decoration: the *gradient* of a field is everywhere perpendicular to its level *surfaces* — the direction of change, and the thing the change is measured against. Surface reports **divergence** between what your docs claim and what your code does.</sub>