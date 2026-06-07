# Authoring hubs

A **hub** is a markdown file whose frontmatter anchors sentences ("claims") to the code they
describe. This guide covers writing claims, the anchor grammar, choosing the right granularity,
and the verify loop. For the end-to-end first run, see the [Quickstart](../../README.md#quickstart).

## Anatomy of a hub

```yaml
---
summary: How auth refresh rotation works.
anchors:
  - claim: refresh rotation is single-use; reuse triggers global logout
    at: src/auth/refresh.ts > rotateRefreshToken
    hash: 9b1c33ade8f1        # written by `surf verify`, not by hand
refs: []
---

# Auth

Prose a human (or agent) reads to understand this domain.
```

- **`claim`** — one sentence stating an invariant. Write what must stay true, not how the code
  is structured. A claim that restates the implementation rots as fast as a comment.
- **`at`** — the anchor: where the claim's logic lives (grammar below).
- **`hash`** — the seal. Absent until you `surf verify`; the gate treats a hashless claim as
  *unverified*.

Where hubs live is configured by the `hubs` glob in `surf.toml` (default `hubs/*.md`); keep them
central or co-locate them with code (`["**/_hub.md"]`).

## The anchor grammar

An anchor is a file path, then a `>`-separated symbol path:

```
src/service.ts > TokenService > rotate
```

- **One segment** points at a top-level symbol: `src/m.rs > parse_anchor`.
- **Nested segments** walk into scopes: a type and its `impl`/methods share a name, so
  `Type` alone may be ambiguous while `Type > method` is unique.
- **`@N`** disambiguates genuine name collisions (1-based), e.g. two overloads:
  `src/api.ts > handler@2`.
- **Multiple sites** — an `at:` list combines its sites into one hash, so the claim is stale if
  *any* listed span changes:
  ```yaml
  at:
    - src/a.rs > foo
    - src/b.rs > bar
  ```

Run `surf lint` to confirm every anchor resolves to exactly one symbol. Ambiguous or vanished
anchors **block**; a symbol that was merely renamed only **warns** and points you at
`surf verify --follow`.

## Choosing granularity

This is the central tension (proposal §8):

- **Under-anchor** → real drift slips through, because the changed logic wasn't anchored.
- **Over-anchor** → every incidental edit re-triggers verification, and humans start
  rubber-stamping `verify` without reading — which defeats the tool.

`surf lint` emits advisory warnings (never blocking) to nudge you toward the middle:

- **Near-whole-file span** — the anchored symbol covers most of its file. Anchor a narrower
  symbol so unrelated edits don't trip the claim.
- **Too many anchors in one hub** — split the hub; a long verify list invites rubber-stamping.
- **Uncovered public function** — a public function in a file the hub already anchors has no
  claim. Either add one, or accept it as intentionally undocumented.

Rule of thumb: anchor the **smallest symbol whose logic the sentence is actually about.**

## The verify loop

`surf verify` is the human escape hatch: it re-seals a claim after *you* confirm the prose still
holds, writing the hash into the frontmatter (and touching only that line).

```sh
surf check                      # DIVERGED? a claim's anchored logic changed
# re-read the claim:
#   still true  → surf verify [<at>]      (re-seal)
#   now false   → fix the prose first, then verify
surf verify --follow            # renamed symbol: re-point the anchor and re-hash in one step
```

Verifying without reading is the failure mode the whole tool exists to prevent. A green gate
promises only "nothing anchored changed since last sign-off" — never that the prose is true.

See also: [CI integration](./ci-integration.md) · [Examples](../examples.md).
