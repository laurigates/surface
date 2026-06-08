---
summary: The CLI command/flag surface — the clap `Command` enum that `docs/reference/commands.md` documents.
anchors:
  - claim: >
      The CLI exposes exactly these subcommands with these flags: init; new <name>; lint
      [--format]; check [--format] [--base <ref>] [--files <globs>]; verify [<target>] [--follow]
      [--format]; suggest <globs> [--format]. Adding, removing, or renaming a command or flag, or
      changing a default, diverges this anchor — re-read docs/reference/commands.md before sealing.
    at: surf-cli/src/main.rs > Command
    hash: 9b09340872e6
refs: ["../docs/reference/commands.md"]
---

# CLI reference surface

`Command` is the clap-derived definition of every `surf` subcommand and flag. It is the source of
truth that [`docs/reference/commands.md`](../docs/reference/commands.md) describes in prose, so the
gate fails when the two drift.
