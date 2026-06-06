use clap::{Parser, Subcommand};

mod workspace;

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
    /// Validate hub frontmatter and that every anchor resolves to exactly one symbol.
    Lint,
    /// The gate: hash each anchored span and block on any documented span that diverged.
    Check,
    /// Re-hash an anchor after a human confirms the prose still holds.
    Verify,
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match run(&cli.command) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(command: &Command) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = Workspace::discover(&cwd)?;
    let hubs = ws.hub_paths()?;

    let name = match command {
        Command::Lint => "lint",
        Command::Check => "check",
        Command::Verify => "verify",
    };
    println!(
        "surf {name}: discovered workspace at {} ({} hub(s)); command not implemented yet",
        ws.root.display(),
        hubs.len()
    );
    Ok(())
}
