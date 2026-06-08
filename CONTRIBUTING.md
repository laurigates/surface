# Contributing to Surface

## Prerequisites

Rust, via [rustup](https://rustup.rs). The toolchain is pinned in `rust-toolchain.toml`, so
`rustup` installs the right version automatically the first time you build.

Optionally install [pre-commit](https://pre-commit.com) and run `pre-commit install` once.
This wires up the local `surf lint`/`surf check` hooks (`.pre-commit-config.yaml`) so docs ↔
code drift is caught at commit time, the same gate CI runs.

## Build & test

```sh
cargo build                                      # build the workspace
cargo test --all                                 # run all tests
cargo fmt --all                                  # format
cargo clippy --all-targets -- -D warnings        # lint (CI fails on any warning)
```

## Run it on this repo (dogfood)

Surface governs its own `surf-core`:

```sh
cargo run -q -p surf-cli -- lint    # every anchor resolves
cargo run -q -p surf-cli -- check   # anchored spans match their stored hashes
```

If you change a symbol that a hub anchors (see `hubs/`), `check` will block until you either
revert or — if the change is intended and the prose still holds — re-stamp it:

```sh
cargo run -q -p surf-cli -- verify "surf-core/src/hash.rs > emit"
```

## Layout

- `surf-core/` — pure parse/resolve/hash logic, no I/O (also the future WASM target).
- `surf-cli/` — the `surf` binary: workspace discovery, the commands, all I/O.
- `docs/phases/` — how the MVP was built, one self-contained file per phase. Start with
  `docs/phases/OVERVIEW.md`. The product spec is `docs/surface-proposal.md`.
- `docs/index.md` — the documentation overview; `docs/getting-started/`, `docs/guides/`, and
  `docs/reference/` hold the user-facing pages. `AGENTS.md` is the on-ramp for AI coding agents.

Keep `surf-core` free of I/O so it stays reusable; put filesystem/git work in `surf-cli`.

**Docs source of truth.** This repo's `docs/` is canonical. The Starlight docs site
([`Connorrmcd6/surface-site`](https://github.com/Connorrmcd6/surface-site),
surface.gradientdev.xyz) is updated *from* these pages — edit docs here, never only on the site.

When a change is user-facing, add a line to `CHANGELOG.md` under `[Unreleased]`.
