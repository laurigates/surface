//! The hub document: `---`-fenced YAML frontmatter + a markdown prose body (§9.1.1).
//! This module is pure: it parses a string into a `Hub`. It does no I/O and resolves no
//! anchors — that is `lint`/`check`'s job over the data this produces.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hub {
    pub frontmatter: Frontmatter,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frontmatter {
    pub summary: String,
    #[serde(default)]
    pub anchors: Vec<Claim>,
    /// Hub composition. Forward-declared per §9.3 — parsed but inert in the MVP.
    #[serde(default)]
    pub refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub claim: String,
    pub at: At,
    /// The stored AST-canonical hash. `None` until `surf verify` first stamps it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

/// One anchor (`at:`) is either a single span or a list; the claim is stale if *any*
/// listed span changes (§6.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum At {
    One(String),
    Many(Vec<String>),
}

impl At {
    pub fn sites(&self) -> &[String] {
        match self {
            At::One(s) => std::slice::from_ref(s),
            At::Many(v) => v,
        }
    }
}

#[derive(Debug)]
pub enum HubError {
    MissingFrontmatter,
    UnterminatedFrontmatter,
    Yaml(String),
}

impl std::fmt::Display for HubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HubError::MissingFrontmatter => {
                write!(
                    f,
                    "hub must begin with a `---`-fenced YAML frontmatter block"
                )
            }
            HubError::UnterminatedFrontmatter => {
                write!(f, "frontmatter block is not closed with `---`")
            }
            HubError::Yaml(e) => write!(f, "invalid frontmatter: {e}"),
        }
    }
}

impl std::error::Error for HubError {}

pub fn parse_hub(content: &str) -> Result<Hub, HubError> {
    let mut lines = content.lines();
    match lines.next() {
        Some(first) if first.trim_end() == "---" => {}
        _ => return Err(HubError::MissingFrontmatter),
    }

    let mut yaml = String::new();
    let mut closed = false;
    for line in lines.by_ref() {
        if line.trim_end() == "---" {
            closed = true;
            break;
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    if !closed {
        return Err(HubError::UnterminatedFrontmatter);
    }

    let frontmatter: Frontmatter =
        serde_yaml::from_str(&yaml).map_err(|e| HubError::Yaml(e.to_string()))?;
    let body = lines.collect::<Vec<_>>().join("\n");

    Ok(Hub { frontmatter, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "---\nsummary: how auth refresh works\nanchors:\n  - claim: refresh rotation is single-use\n    at: src/auth/refresh.ts > rotateRefreshToken\n    hash: 9b1c33a\n  - claim: a refresh token is accepted at most once\n    at:\n      - src/auth/refresh.ts > rotateRefreshToken\n      - src/auth/refresh.ts > validateRefresh\nrefs: []\n---\n# Auth\n\nProse body here.\n";

    #[test]
    fn parses_scalar_and_list_at() {
        let hub = parse_hub(VALID).unwrap();
        assert_eq!(hub.frontmatter.summary, "how auth refresh works");
        assert_eq!(hub.frontmatter.anchors.len(), 2);

        let first = &hub.frontmatter.anchors[0];
        assert_eq!(
            first.at.sites(),
            &["src/auth/refresh.ts > rotateRefreshToken".to_string()]
        );
        assert_eq!(first.hash.as_deref(), Some("9b1c33a"));

        let second = &hub.frontmatter.anchors[1];
        assert_eq!(second.at.sites().len(), 2);
        assert_eq!(second.hash, None);

        assert!(hub.body.contains("Prose body here."));
    }

    #[test]
    fn round_trips_frontmatter() {
        let hub = parse_hub(VALID).unwrap();
        let yaml = serde_yaml::to_string(&hub.frontmatter).unwrap();
        let reparsed: Frontmatter = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(hub.frontmatter, reparsed);
    }

    #[test]
    fn missing_frontmatter_is_typed_error() {
        let err = parse_hub("# Just markdown, no frontmatter\n").unwrap_err();
        assert!(matches!(err, HubError::MissingFrontmatter));
    }

    #[test]
    fn unterminated_frontmatter_is_typed_error() {
        let err = parse_hub("---\nsummary: x\nstill inside\n").unwrap_err();
        assert!(matches!(err, HubError::UnterminatedFrontmatter));
    }

    #[test]
    fn missing_required_field_is_yaml_error() {
        let err = parse_hub("---\nanchors: []\n---\nbody\n").unwrap_err();
        assert!(
            matches!(err, HubError::Yaml(_)),
            "expected Yaml error, got {err:?}"
        );
    }

    #[test]
    fn covers_field_is_rejected() {
        // `covers` is deliberately absent from the MVP schema (§9.1); deny_unknown_fields
        // surfaces it as a clear error rather than silently ignoring a field that does nothing.
        let err = parse_hub("---\nsummary: x\ncovers:\n  - src/**\n---\nbody\n").unwrap_err();
        assert!(matches!(err, HubError::Yaml(_)));
    }

    #[test]
    fn refs_parse_without_resolution() {
        let hub = parse_hub("---\nsummary: x\nrefs:\n  - other-hub\n---\nbody\n").unwrap();
        assert_eq!(hub.frontmatter.refs, vec!["other-hub".to_string()]);
    }
}
