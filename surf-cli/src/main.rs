use clap::{Parser, Subcommand};

mod check;
mod lint;
mod new;
mod verify;
mod workspace;

use check::Format;
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
    /// Scaffold a new, empty hub under the configured hubs directory.
    New {
        /// Hub name; creates `<hubs-dir>/<name>.md`.
        name: String,
    },
    /// Validate hub frontmatter and that every anchor resolves to exactly one symbol.
    Lint,
    /// The gate: hash each anchored span and block on any documented span that diverged.
    Check {
        /// Output format for the divergence report.
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
        /// Git ref to recover previous code from for advisory old_code/magnitude.
        #[arg(long, default_value = "HEAD")]
        base: String,
    },
    /// Re-hash an anchor after a human confirms the prose still holds.
    Verify {
        /// Only verify the anchor whose `at:` exactly matches this (default: all anchors).
        target: Option<String>,
        /// Re-point a renamed single-segment anchor to its new symbol, then re-hash.
        #[arg(long)]
        follow: bool,
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
    let ws = Workspace::discover(&cwd)?;

    match cli.command {
        Command::New { name } => new::run(&ws, &name),
        Command::Lint => lint::run(&ws),
        Command::Check { format, base } => check::run(&ws, format, &base),
        Command::Verify { target, follow } => verify::run(&ws, target.as_deref(), follow),
    }
}
