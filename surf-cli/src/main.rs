use clap::{Parser, Subcommand};

mod check;
mod format;
mod init;
mod lint;
mod new;
mod suggest;
mod verify;
mod workspace;

use format::Format;
use workspace::Workspace;

const SCOPE_DISCLAIMER: &str = "\
Surface checks that the code a claim points at is unchanged since it was last verified.
It does NOT verify that the documented invariant still holds across the system: a change
elsewhere can falsify a claim while its anchored span — and this gate — stays green.";

#[derive(Parser)]
#[command(
    name = "surf",
    version,
    about = "Surface — a deterministic gate that surfaces divergence between docs and code.",
    long_about = SCOPE_DISCLAIMER
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Bootstrap a workspace: write surf.toml and create the hubs directory.
    Init,
    /// Scaffold a new, empty hub under the configured hubs directory.
    New {
        /// Hub name; creates `<hubs-dir>/<name>.md`.
        name: String,
    },
    /// Validate hub frontmatter and that every anchor resolves to exactly one symbol.
    Lint {
        /// Output format for the findings.
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },
    /// The gate: hash each anchored span and block on any documented span that diverged.
    Check {
        /// Output format for the divergence report.
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
        /// Git ref to diff against: scopes the check to claims whose files changed since the
        /// merge base, and recovers previous code for advisory old_code/magnitude. Omit for a
        /// full check (enrichment falls back to HEAD).
        #[arg(long)]
        base: Option<String>,
        /// Only evaluate claims whose anchored file(s) match one of these globs.
        #[arg(long, value_delimiter = ',')]
        files: Vec<String>,
    },
    /// Re-hash an anchor after a human confirms the prose still holds.
    Verify {
        /// Only verify the anchor whose `at:` exactly matches this (default: all anchors).
        target: Option<String>,
        /// Re-point a renamed single-segment anchor to its new symbol, then re-hash.
        #[arg(long)]
        follow: bool,
        /// Output format for the verify report.
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },
    /// Propose anchors for public functions no hub covers yet (suggestions only; never writes).
    Suggest {
        /// Source globs to scan, relative to the workspace root (e.g. "surf-core/src/**/*.rs").
        #[arg(required = true)]
        globs: Vec<String>,
        /// Output format for the suggestions.
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<std::process::ExitCode> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir()?;

    // `init` creates the surf.toml marker, so it runs before (and instead of) discovery.
    if matches!(cli.command, Command::Init) {
        return init::run(&cwd);
    }

    let ws = Workspace::discover(&cwd)?;
    match cli.command {
        Command::Init => unreachable!("handled before discovery"),
        Command::New { name } => new::run(&ws, &name),
        Command::Lint { format } => lint::run(&ws, format),
        Command::Check {
            format,
            base,
            files,
        } => check::run(&ws, format, base.as_deref(), &files),
        Command::Verify {
            target,
            follow,
            format,
        } => verify::run(&ws, target.as_deref(), follow, format),
        Command::Suggest { globs, format } => suggest::run(&ws, &globs, format),
    }
}
