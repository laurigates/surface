//! `surf init` — bootstrap a workspace: write `surf.toml` (the root marker every other
//! command discovers) and create the hubs directory. This is the one command that can't
//! discover a workspace first, because it's the command that creates one. Idempotent: an
//! existing `surf.toml` is left untouched.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::ExitCode;
use surf_core::CONFIG_FILE;

const DEFAULT_CONFIG: &str = "hubs = [\"hubs/*.md\"]\n";

pub fn run(cwd: &Path) -> Result<ExitCode> {
    let config = cwd.join(CONFIG_FILE);
    if config.exists() {
        println!("surf: already initialized ({CONFIG_FILE} exists)");
        return Ok(ExitCode::SUCCESS);
    }

    std::fs::write(&config, DEFAULT_CONFIG)
        .with_context(|| format!("writing {}", config.display()))?;
    let hubs = cwd.join("hubs");
    std::fs::create_dir_all(&hubs).with_context(|| format!("creating {}", hubs.display()))?;

    println!("created {CONFIG_FILE} and hubs/");
    println!("next: `surf new <name>` to scaffold your first hub.");
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn creates_config_and_hubs_dir() {
        let tmp = tempfile::tempdir().unwrap();
        run(tmp.path()).unwrap();
        assert!(tmp.path().join("surf.toml").is_file());
        assert!(tmp.path().join("hubs").is_dir());
    }

    #[test]
    fn does_not_clobber_existing_config() {
        let tmp = tempfile::tempdir().unwrap();
        let existing = "hubs = [\"custom/*.md\"]\n";
        fs::write(tmp.path().join("surf.toml"), existing).unwrap();
        run(tmp.path()).unwrap();
        assert_eq!(
            fs::read_to_string(tmp.path().join("surf.toml")).unwrap(),
            existing
        );
    }
}
