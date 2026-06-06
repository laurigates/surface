//! Resolve an anchor to the exact span of the named symbol via tree-sitter (§6.1, §6.3).
//!
//! Resolution walks segment by segment. Each segment is matched against the symbol
//! definitions reachable in the *current* scope set; matched containers become the next
//! scope set. A scope is a *set* of nodes, not one node, so a type and its `impl` block
//! (which share a name) both get descended — the path `Type > method` resolves uniquely
//! even though `Type` alone is ambiguous.

use crate::anchor::{Anchor, Segment};
use crate::lang::{Family, Lang};
use tree_sitter::{Node, Parser, Tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    Parse,
    NotFound { segment: String },
    Ambiguous { segment: String, count: usize },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::Parse => write!(f, "source could not be parsed"),
            ResolveError::NotFound { segment } => {
                write!(f, "no symbol `{segment}` found at this path")
            }
            ResolveError::Ambiguous { segment, count } => {
                write!(
                    f,
                    "`{segment}` is ambiguous ({count} matches); disambiguate with `@N`"
                )
            }
        }
    }
}

impl std::error::Error for ResolveError {}

pub fn resolve(source: &str, lang: Lang, anchor: &Anchor) -> Result<Span, ResolveError> {
    let tree = parse_tree(source, lang).ok_or(ResolveError::Parse)?;
    let node = resolve_node(tree.root_node(), source.as_bytes(), lang.family(), anchor)?;
    Ok(span_of(node))
}

pub(crate) fn parse_tree(source: &str, lang: Lang) -> Option<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang.tree_sitter_language())
        .expect("bundled grammar is always a valid language");
    parser.parse(source, None)
}

pub(crate) fn resolve_node<'a>(
    root: Node<'a>,
    src: &[u8],
    family: Family,
    anchor: &Anchor,
) -> Result<Node<'a>, ResolveError> {
    // Go symbols are flat (no nested declarations) and methods attach to a type by receiver,
    // not by nesting — so it gets a dedicated resolver rather than the generic scope walk.
    if family == Family::Go {
        return resolve_go(root, src, anchor);
    }

    let mut scopes = vec![root];

    let last = anchor.segments.len() - 1;
    for (i, seg) in anchor.segments.iter().enumerate() {
        let mut matches: Vec<Node> = Vec::new();
        for scope in &scopes {
            collect_matching(*scope, src, family, &seg.name, &mut matches);
        }

        let selected: Vec<Node> = match seg.index {
            Some(k) => matches.get(k - 1).copied().into_iter().collect(),
            None => matches,
        };

        match selected.len() {
            0 => {
                return Err(ResolveError::NotFound {
                    segment: seg.name.clone(),
                })
            }
            1 if i == last => return Ok(selected[0]),
            n if i == last => {
                return Err(ResolveError::Ambiguous {
                    segment: seg.name.clone(),
                    count: n,
                })
            }
            _ => scopes = selected.iter().map(|n| scope_of(*n, family)).collect(),
        }
    }

    unreachable!("an anchor always has at least one segment")
}

/// Apply a segment's positional/uniqueness rule to a candidate list.
fn pick<'a>(candidates: Vec<Node<'a>>, seg: &Segment) -> Result<Node<'a>, ResolveError> {
    let selected: Vec<Node> = match seg.index {
        Some(k) => candidates.get(k - 1).copied().into_iter().collect(),
        None => candidates,
    };
    match selected.len() {
        0 => Err(ResolveError::NotFound {
            segment: seg.name.clone(),
        }),
        1 => Ok(selected[0]),
        n => Err(ResolveError::Ambiguous {
            segment: seg.name.clone(),
            count: n,
        }),
    }
}

/// Go resolver. `file.go > Fn|Type` for top-level declarations; `file.go > Type > Method`
/// for methods, matched by receiver type (Go methods are flat, not nested in the type).
fn resolve_go<'a>(root: Node<'a>, src: &[u8], anchor: &Anchor) -> Result<Node<'a>, ResolveError> {
    let segs = &anchor.segments;
    match segs.as_slice() {
        [one] => pick(go_top_level(root, src, &one.name), one),
        [ty, method] => {
            let candidates = go_methods(root, src, &ty.name)
                .into_iter()
                .filter(|(name, _)| *name == method.name)
                .map(|(_, node)| node)
                .collect();
            pick(candidates, method)
        }
        _ => Err(ResolveError::NotFound {
            segment: segs.last().expect("anchor has >= 1 segment").name.clone(),
        }),
    }
}

fn go_top_level<'a>(node: Node<'a>, src: &[u8], name: &str) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "function_declaration" | "type_spec" | "const_spec" | "var_spec" => {
                if field_text(child, "name", src) == Some(name) {
                    out.push(child);
                }
            }
            "type_declaration" | "const_declaration" | "var_declaration" => {
                out.extend(go_top_level(child, src, name));
            }
            _ => {}
        }
    }
    out
}

fn go_methods<'a>(root: Node<'a>, src: &[u8], type_name: &str) -> Vec<(String, Node<'a>)> {
    let mut out = Vec::new();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if child.kind() == "method_declaration" && go_receiver_type(child, src) == Some(type_name) {
            if let Some(name) = field_text(child, "name", src) {
                out.push((name.to_string(), child));
            }
        }
    }
    out
}

fn go_receiver_type<'a>(method: Node, src: &'a [u8]) -> Option<&'a str> {
    let receiver = method.child_by_field_name("receiver")?;
    let mut cursor = receiver.walk();
    let params: Vec<Node> = receiver.named_children(&mut cursor).collect();
    let param = params
        .into_iter()
        .find(|p| p.kind() == "parameter_declaration")?;
    go_type_name(param.child_by_field_name("type")?, src)
}

fn go_type_name<'a>(ty: Node, src: &'a [u8]) -> Option<&'a str> {
    match ty.kind() {
        "type_identifier" => ty.utf8_text(src).ok(),
        "pointer_type" | "generic_type" => {
            let mut cursor = ty.walk();
            let children: Vec<Node> = ty.named_children(&mut cursor).collect();
            children.into_iter().find_map(|c| go_type_name(c, src))
        }
        _ => None,
    }
}

fn span_of(node: Node) -> Span {
    Span {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
    }
}

fn collect_matching<'a>(
    scope: Node<'a>,
    src: &[u8],
    family: Family,
    name: &str,
    out: &mut Vec<Node<'a>>,
) {
    let mut cursor = scope.walk();
    for child in scope.named_children(&mut cursor) {
        if let Some(def_name) = def_name(child, src, family) {
            if def_name == name {
                out.push(child);
            }
        } else if is_transparent(child.kind(), family) {
            collect_matching(child, src, family, name, out);
        }
    }
}

/// Every symbol definition in the subtree, at any depth, as (name, node). Used for
/// hash-based rename detection — find a current symbol whose canonical hash matches a
/// claim's stored hash even though its name no longer matches the anchor.
pub(crate) fn collect_all_defs<'a>(
    node: Node<'a>,
    src: &[u8],
    family: Family,
    out: &mut Vec<(String, Node<'a>)>,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = def_name(child, src, family) {
            out.push((name, child));
            collect_all_defs(scope_of(child, family), src, family, out);
        } else {
            collect_all_defs(child, src, family, out);
        }
    }
}

fn field_text<'a>(node: Node, field: &str, src: &'a [u8]) -> Option<&'a str> {
    node.child_by_field_name(field)?.utf8_text(src).ok()
}

fn def_name(node: Node, src: &[u8], family: Family) -> Option<String> {
    match family {
        Family::Rust => rust_def_name(node, src),
        Family::TypeScript => ts_def_name(node, src),
        Family::Python => python_def_name(node, src),
        Family::Go => go_def_name(node, src),
    }
}

fn python_def_name(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "function_definition" | "class_definition" => {
            field_text(node, "name", src).map(str::to_string)
        }
        _ => None,
    }
}

fn go_def_name(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "function_declaration" | "method_declaration" | "type_spec" | "const_spec" | "var_spec" => {
            field_text(node, "name", src).map(str::to_string)
        }
        _ => None,
    }
}

fn rust_def_name(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "function_item" | "struct_item" | "enum_item" | "union_item" | "trait_item"
        | "mod_item" | "type_item" | "const_item" | "static_item" | "macro_definition" => {
            field_text(node, "name", src).map(str::to_string)
        }
        "impl_item" => field_text(node, "type", src).map(str::to_string),
        _ => None,
    }
}

fn ts_def_name(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "function_declaration"
        | "generator_function_declaration"
        | "function_signature"
        | "class_declaration"
        | "abstract_class_declaration"
        | "interface_declaration"
        | "enum_declaration"
        | "type_alias_declaration"
        | "method_definition"
        | "method_signature"
        | "abstract_method_signature" => field_text(node, "name", src).map(str::to_string),
        "variable_declarator" => {
            let value = node.child_by_field_name("value")?;
            matches!(
                value.kind(),
                "arrow_function" | "function" | "function_expression" | "generator_function"
            )
            .then(|| field_text(node, "name", src).map(str::to_string))
            .flatten()
        }
        _ => None,
    }
}

fn is_transparent(kind: &str, family: Family) -> bool {
    match family {
        Family::Rust => matches!(kind, "source_file" | "declaration_list" | "block"),
        Family::TypeScript => matches!(
            kind,
            "program"
                | "statement_block"
                | "class_body"
                | "export_statement"
                | "lexical_declaration"
                | "variable_declaration"
        ),
        Family::Python => matches!(kind, "module" | "block" | "decorated_definition"),
        Family::Go => matches!(
            kind,
            "source_file" | "block" | "type_declaration" | "const_declaration" | "var_declaration"
        ),
    }
}

fn scope_of(node: Node, family: Family) -> Node {
    match family {
        Family::TypeScript if node.kind() == "variable_declarator" => {
            node.child_by_field_name("value").unwrap_or(node)
        }
        _ => node.child_by_field_name("body").unwrap_or(node),
    }
}
