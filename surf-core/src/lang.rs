//! Supported languages and their bundled tree-sitter grammars (§6.1).
//! Grammars are compiled into the binary and version-pinned in Cargo.toml — the
//! reproducibility root. Adding a language is additive here.

use std::path::Path;
use tree_sitter::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    TypeScript,
    Tsx,
    Rust,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Family {
    TypeScript,
    Rust,
}

impl Lang {
    pub fn from_path(path: &str) -> Option<Lang> {
        let ext = Path::new(path).extension()?.to_str()?;
        match ext {
            "ts" | "mts" | "cts" => Some(Lang::TypeScript),
            "tsx" => Some(Lang::Tsx),
            "rs" => Some(Lang::Rust),
            _ => None,
        }
    }

    pub(crate) fn tree_sitter_language(self) -> Language {
        match self {
            Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Lang::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Lang::Rust => tree_sitter_rust::LANGUAGE.into(),
        }
    }

    pub(crate) fn family(self) -> Family {
        match self {
            Lang::TypeScript | Lang::Tsx => Family::TypeScript,
            Lang::Rust => Family::Rust,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_by_extension() {
        assert_eq!(
            Lang::from_path("src/auth/refresh.ts"),
            Some(Lang::TypeScript)
        );
        assert_eq!(Lang::from_path("App.tsx"), Some(Lang::Tsx));
        assert_eq!(Lang::from_path("src/main.rs"), Some(Lang::Rust));
        assert_eq!(Lang::from_path("README.md"), None);
        assert_eq!(Lang::from_path("Makefile"), None);
    }
}
