//! Filesystem side of config: discover `surf.toml` by walking up from a starting
//! directory (like `git`/`ruff`, §9.1.5), then enumerate hub files via its globs.
//! This is the I/O layer that `surf-core`'s pure parsers feed into.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use surf_core::config::{parse_config, Config, CONFIG_FILE};

pub struct Workspace {
    pub root: PathBuf,
    pub config: Config,
}

impl Workspace {
    pub fn discover(start: &Path) -> Result<Workspace> {
        for dir in start.ancestors() {
            let candidate = dir.join(CONFIG_FILE);
            if candidate.is_file() {
                let content = std::fs::read_to_string(&candidate)
                    .with_context(|| format!("reading {}", candidate.display()))?;
                let config = parse_config(&content)?;
                return Ok(Workspace {
                    root: dir.to_path_buf(),
                    config,
                });
            }
        }
        anyhow::bail!(
            "no {CONFIG_FILE} found in {} or any parent directory",
            start.display()
        )
    }

    pub fn hub_paths(&self) -> Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        for pattern in &self.config.hubs {
            let joined = self.root.join(pattern);
            let pattern = joined
                .to_str()
                .with_context(|| format!("hub glob is not valid UTF-8: {}", joined.display()))?;
            for entry in glob::glob(pattern).context("invalid hub glob pattern")? {
                out.push(entry?);
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn discovers_config_from_nested_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join(CONFIG_FILE), "hubs = [\"hubs/*.md\"]\n").unwrap();

        let nested = root.join("a/b/c");
        fs::create_dir_all(&nested).unwrap();

        let ws = Workspace::discover(&nested).unwrap();
        assert_eq!(
            ws.root.canonicalize().unwrap(),
            root.canonicalize().unwrap()
        );
        assert_eq!(ws.config.hubs, vec!["hubs/*.md".to_string()]);
    }

    #[test]
    fn errors_when_no_config_anywhere() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(Workspace::discover(tmp.path()).is_err());
    }

    #[test]
    fn globs_hub_files_relative_to_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join(CONFIG_FILE), "").unwrap();
        fs::create_dir_all(root.join("hubs")).unwrap();
        fs::write(root.join("hubs/auth.md"), "---\nsummary: x\n---\n").unwrap();
        fs::write(root.join("hubs/billing.md"), "---\nsummary: y\n---\n").unwrap();
        fs::write(root.join("hubs/notes.txt"), "ignored").unwrap();

        let ws = Workspace::discover(root).unwrap();
        let hubs = ws.hub_paths().unwrap();
        let names: Vec<_> = hubs
            .iter()
            .filter_map(|p| p.file_name()?.to_str())
            .collect();
        assert_eq!(names, vec!["auth.md", "billing.md"]);
    }
}
