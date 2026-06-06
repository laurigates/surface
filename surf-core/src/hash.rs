//! AST-canonical hashing (§6.1) and advisory diff magnitude (§6.2).
//!
//! The hash is computed over a canonical token stream of the symbol's subtree:
//! - whitespace and formatting are absent from the tree, so they are ignored for free;
//! - comments are dropped explicitly;
//! - identifiers are alpha-renamed to positional placeholders (`#0`, `#1`, …) in order of
//!   first occurrence, so a *consistent* rename hashes identically while swapping two names
//!   does not;
//! - operators, keywords, punctuation, and literal *values* are kept verbatim — so a
//!   flipped operator (`+`→`-`), a relaxed comparison (`<`→`<=`), a deleted `await`, or a
//!   changed constant all change the hash.
//!
//! The result is quiet on the changes you want ignored and loud on the ones you must catch.
//!
//! `Magnitude` is advisory triage metadata only. It is never compared, thresholded, or used
//! to decide pass/fail — that would defeat the whole point (§6.2).

use crate::anchor::Anchor;
use crate::lang::{Family, Lang};
use crate::resolve::{parse_tree, resolve_node, ResolveError};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::Write as _;
use tree_sitter::Node;

const HASH_HEX_LEN: usize = 12;

pub fn hash_anchor(source: &str, lang: Lang, anchor: &Anchor) -> Result<String, ResolveError> {
    Ok(hash_tokens(&anchor_tokens(source, lang, anchor)?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Magnitude {
    Small,
    Medium,
    Large,
}

/// Advisory only — describes how big the change to a span was, for triage. Never gates.
pub fn diff_magnitude(
    old_source: &str,
    new_source: &str,
    lang: Lang,
    anchor: &Anchor,
) -> Result<Magnitude, ResolveError> {
    let old = anchor_tokens(old_source, lang, anchor)?;
    let new = anchor_tokens(new_source, lang, anchor)?;
    Ok(categorize(token_distance(&old, &new)))
}

fn anchor_tokens(source: &str, lang: Lang, anchor: &Anchor) -> Result<Vec<String>, ResolveError> {
    let tree = parse_tree(source, lang).ok_or(ResolveError::Parse)?;
    let src = source.as_bytes();
    let family = lang.family();
    let node = resolve_node(tree.root_node(), src, family, anchor)?;
    Ok(canonical_tokens(node, src, family))
}

fn canonical_tokens(node: Node, src: &[u8], family: Family) -> Vec<String> {
    let mut out = Vec::new();
    let mut idents: HashMap<String, usize> = HashMap::new();
    emit(node, src, family, &mut idents, &mut out);
    out
}

fn emit(
    node: Node,
    src: &[u8],
    family: Family,
    idents: &mut HashMap<String, usize>,
    out: &mut Vec<String>,
) {
    let kind = node.kind();
    if kind.contains("comment") {
        return;
    }

    if node.is_named() {
        if is_identifier(kind, family) {
            let text = node.utf8_text(src).unwrap_or_default();
            let next = idents.len();
            let idx = *idents.entry(text.to_string()).or_insert(next);
            out.push(format!("#{idx}"));
            return;
        }
        if node.child_count() == 0 {
            // Named terminal (literal, primitive type, keyword-like): keep its value.
            out.push(format!(
                "{kind}:{}",
                node.utf8_text(src).unwrap_or_default()
            ));
            return;
        }
        out.push(kind.to_string());
    } else {
        // Anonymous token: operator, punctuation, or keyword. Its kind *is* the text.
        out.push(kind.to_string());
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        emit(child, src, family, idents, out);
    }
}

fn is_identifier(kind: &str, family: Family) -> bool {
    match family {
        Family::Rust => matches!(
            kind,
            "identifier" | "type_identifier" | "field_identifier" | "shorthand_field_identifier"
        ),
        Family::TypeScript => matches!(
            kind,
            "identifier"
                | "type_identifier"
                | "property_identifier"
                | "shorthand_property_identifier"
                | "shorthand_property_identifier_pattern"
                | "private_property_identifier"
        ),
    }
}

fn hash_tokens(tokens: &[String]) -> String {
    let mut hasher = Sha256::new();
    for t in tokens {
        hasher.update(t.as_bytes());
        hasher.update([0u8]);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(HASH_HEX_LEN);
    for b in digest.iter().take(HASH_HEX_LEN / 2) {
        write!(hex, "{b:02x}").expect("writing to a String never fails");
    }
    hex
}

fn token_distance(a: &[String], b: &[String]) -> usize {
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut curr = vec![0usize; m + 1];
    for i in 1..=n {
        curr[0] = i;
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[m]
}

fn categorize(distance: usize) -> Magnitude {
    match distance {
        0..=3 => Magnitude::Small,
        4..=15 => Magnitude::Medium,
        _ => Magnitude::Large,
    }
}
