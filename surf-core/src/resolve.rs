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
    let family = lang.family();
    let node = resolve_node(tree.root_node(), source.as_bytes(), family, anchor)?;
    Ok(span_of(hashable_node(node, family)))
}

/// The node whose span/tokens represent a resolved symbol. Resolution keys off the inner
/// definition node (it carries the `name`), but in tree-sitter-python a decorated
/// function/class excludes its decorators — they live in the parent `decorated_definition`.
/// Widen to that parent so a decorator-only change is part of the hash (§6.1). Applied at the
/// two chokepoints (span here, tokens in `hash`) so every hash of the same symbol agrees.
pub(crate) fn hashable_node(node: Node, family: Family) -> Node {
    match family {
        Family::Python if matches!(node.kind(), "function_definition" | "class_definition") => node
            .parent()
            .filter(|p| p.kind() == "decorated_definition")
            .unwrap_or(node),
        _ => node,
    }
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

/// Top-level public *functions* on a file's surface: unrestricted `pub fn` in Rust,
/// `export`ed functions in TS/JS, non-underscore `def`s in Python, capitalized `func`s in Go.
/// Advisory input to lint's under-coverage warning. Deliberately functions only — a claim
/// documents *behavior* that can drift, so pure data types (structs/enums/consts) are out of
/// scope to avoid the over-anchoring fatigue of §8. Shallow (top-level) and best-effort.
pub fn public_fns(source: &str, lang: Lang) -> Vec<String> {
    let Some(tree) = parse_tree(source, lang) else {
        return Vec::new();
    };
    let src = source.as_bytes();
    let family = lang.family();
    let mut out = Vec::new();
    let root = tree.root_node();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        collect_public_fn(child, src, family, &mut out);
    }
    out.sort();
    out.dedup();
    out
}

/// Which slice of a file's public surface [`public_symbols`] enumerates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// Behaviour-bearing only: top-level functions and the methods that make up most of a
    /// Python/Go API. A claim documents behaviour that can drift, so pure data/types are out of
    /// scope. This is what `lint`'s under-coverage nudge measures and what `suggest` proposes by
    /// default.
    Callables,
    /// Everything anchorable: additionally top-level classes, module-level constants and type
    /// aliases, and class attributes (Python). Drives `suggest --all`, so a user can *discover*
    /// the non-callable targets `resolve` already accepts. Other languages enumerate the same
    /// callables as `Callables` for now.
    All,
}

/// Public symbols on a file's surface as resolvable anchor segment paths: top-level functions
/// (`["foo"]`), and — unlike [`public_fns`] — the methods that make up most of a Python/Go API
/// (`["Builder", "Set"]`). With [`Surface::All`] it also proposes the non-callable Python targets
/// `resolve` accepts: top-level classes (`["C"]`), module-level constants/type aliases
/// (`["CONST"]`), and class attributes (`["C", "attr"]`). With [`Surface::Callables`] those are
/// withheld, keeping the default (and the lint nudge) to behaviour. Methods on Rust `impl` blocks
/// and TS classes remain out of scope; only top-level fns are enumerated for them.
pub fn public_symbols(source: &str, lang: Lang, surface: Surface) -> Vec<Vec<String>> {
    let Some(tree) = parse_tree(source, lang) else {
        return Vec::new();
    };
    let src = source.as_bytes();
    let family = lang.family();
    let mut out = Vec::new();
    let root = tree.root_node();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        collect_public_symbol(child, src, family, surface, &mut out);
    }
    out.sort();
    out.dedup();
    out
}

fn collect_public_symbol(
    node: Node,
    src: &[u8],
    family: Family,
    surface: Surface,
    out: &mut Vec<Vec<String>>,
) {
    match family {
        Family::Rust => {
            if node.kind() == "function_item" && is_rust_pub(node, src) {
                if let Some(name) = field_text(node, "name", src) {
                    out.push(vec![name.to_string()]);
                }
            }
        }
        Family::TypeScript => {
            if node.kind() == "export_statement" {
                let mut names = Vec::new();
                let mut cursor = node.walk();
                for c in node.named_children(&mut cursor) {
                    ts_collect_export_fns(c, src, &mut names);
                }
                out.extend(names.into_iter().map(|n| vec![n]));
            }
        }
        Family::Python => collect_python_symbol(node, src, None, surface, out),
        Family::Go => match node.kind() {
            "function_declaration" => {
                if let Some(name) = field_text(node, "name", src) {
                    if name.chars().next().is_some_and(char::is_uppercase) {
                        out.push(vec![name.to_string()]);
                    }
                }
            }
            // Methods are flat in Go; pair the method with its receiver type so the anchor
            // resolves as `file.go > Type > Method`. Both must be exported to be public surface.
            "method_declaration" => {
                if let (Some(name), Some(ty)) =
                    (field_text(node, "name", src), go_receiver_type(node, src))
                {
                    if name.chars().next().is_some_and(char::is_uppercase)
                        && ty.chars().next().is_some_and(char::is_uppercase)
                    {
                        out.push(vec![ty.to_string(), name.to_string()]);
                    }
                }
            }
            _ => {}
        },
    }
}

/// `class` given: we're inside that class body, so non-underscore `def`s become `[Class, method]`
/// and (under [`Surface::All`]) attributes become `[Class, attr]`. `None`: top level, so functions
/// become `[fn]`, and we descend one level into public classes to surface their methods (nested
/// classes are not recursed into). Under `All` we also emit the class itself (`[Class]`) and
/// module-level constants/type aliases (`[NAME]`). Push helper appends the name under any
/// enclosing class, skipping `_`-prefixed (non-public) names.
fn collect_python_symbol(
    node: Node,
    src: &[u8],
    class: Option<&str>,
    surface: Surface,
    out: &mut Vec<Vec<String>>,
) {
    let mut emit = |name: String| {
        if !name.starts_with('_') {
            let mut path: Vec<String> = class.into_iter().map(str::to_string).collect();
            path.push(name);
            out.push(path);
        }
    };
    match node.kind() {
        "function_definition" => {
            if let Some(name) = python_def_name(node, src) {
                emit(name);
            }
        }
        "class_definition" if class.is_none() => {
            let Some(name) = python_def_name(node, src) else {
                return;
            };
            if name.starts_with('_') {
                return;
            }
            if surface == Surface::All {
                out.push(vec![name.clone()]);
            }
            if let Some(body) = node.child_by_field_name("body") {
                let mut cursor = body.walk();
                for c in body.named_children(&mut cursor) {
                    collect_python_symbol(c, src, Some(&name), surface, out);
                }
            }
        }
        // Module- or class-level non-callable bindings (`CONST = ...`, `X: T = ...`, PEP 695
        // `type X = ...`): anchorable and gateable, but withheld unless `--all` so the default
        // suggestion stays behaviour-focused.
        "assignment" | "type_alias_statement" if surface == Surface::All => {
            if let Some(name) = python_def_name(node, src) {
                emit(name);
            }
        }
        // tree-sitter wraps module/class-body statements (assignments) in `expression_statement`;
        // a decorator wraps the def. Both are transparent — recurse, preserving class + surface.
        "decorated_definition" | "expression_statement" => {
            let mut cursor = node.walk();
            for c in node.named_children(&mut cursor) {
                collect_python_symbol(c, src, class, surface, out);
            }
        }
        _ => {}
    }
}

fn collect_public_fn(node: Node, src: &[u8], family: Family, out: &mut Vec<String>) {
    match family {
        Family::Rust => {
            if node.kind() == "function_item" && is_rust_pub(node, src) {
                if let Some(name) = field_text(node, "name", src) {
                    out.push(name.to_string());
                }
            }
        }
        Family::TypeScript => {
            if node.kind() == "export_statement" {
                let mut cursor = node.walk();
                for c in node.named_children(&mut cursor) {
                    ts_collect_export_fns(c, src, out);
                }
            }
        }
        Family::Python => collect_python_fn(node, src, out),
        Family::Go => {
            if node.kind() == "function_declaration" {
                if let Some(name) = field_text(node, "name", src) {
                    if name.chars().next().is_some_and(char::is_uppercase) {
                        out.push(name.to_string());
                    }
                }
            }
        }
    }
}

/// True only for unrestricted `pub` — `pub(crate)`/`pub(super)`/`pub(in …)` are internal and
/// not part of the file's outward surface.
fn is_rust_pub(node: Node, src: &[u8]) -> bool {
    let mut cursor = node.walk();
    let is_pub = node
        .children(&mut cursor)
        .filter(|c| c.kind() == "visibility_modifier")
        .any(|c| c.utf8_text(src).map(str::trim) == Ok("pub"));
    is_pub
}

fn ts_collect_export_fns(node: Node, src: &[u8], out: &mut Vec<String>) {
    match node.kind() {
        "function_declaration" | "generator_function_declaration" | "function_signature" => {
            if let Some(name) = field_text(node, "name", src) {
                out.push(name.to_string());
            }
        }
        // `export const f = () => {}` — only the function-valued declarators.
        "lexical_declaration" | "variable_declaration" => {
            let mut cursor = node.walk();
            for c in node.named_children(&mut cursor) {
                if let Some(name) = ts_def_name(c, src) {
                    out.push(name);
                }
            }
        }
        _ => {}
    }
}

fn collect_python_fn(node: Node, src: &[u8], out: &mut Vec<String>) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = python_def_name(node, src) {
                if !name.starts_with('_') {
                    out.push(name);
                }
            }
        }
        "decorated_definition" => {
            let mut cursor = node.walk();
            for c in node.named_children(&mut cursor) {
                collect_python_fn(c, src, out);
            }
        }
        _ => {}
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
        // Module- or class-level bindings: `X = ...`, `X: T = ...`, `X: T`. Only a bare
        // identifier target is anchorable — tuple/attribute targets (`a, b = ...`, `self.x = ...`)
        // have no single unambiguous name.
        "assignment" => {
            let left = node.child_by_field_name("left")?;
            (left.kind() == "identifier")
                .then(|| left.utf8_text(src).ok())
                .flatten()
                .map(str::to_string)
        }
        // PEP 695 `type X = ...` — `left` is a `type` wrapping the alias name.
        "type_alias_statement" => node
            .child_by_field_name("left")?
            .utf8_text(src)
            .ok()
            .map(str::to_string),
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
                "arrow_function"
                    | "function"
                    | "function_expression"
                    | "generator_function"
                    | "call_expression"
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
        Family::Python => matches!(
            kind,
            "module" | "block" | "decorated_definition" | "expression_statement"
        ),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn syms(source: &str, lang: Lang) -> Vec<String> {
        public_fns(source, lang)
    }

    #[test]
    fn rust_only_unrestricted_pub_fns() {
        // Functions only; private, `pub(crate)`, and non-fn items (struct/enum/impl) excluded.
        let src = "pub fn a() {}\nfn b() {}\npub(crate) fn c() {}\npub struct S;\nenum E {}\nimpl S { pub fn m(&self) {} }\n";
        assert_eq!(syms(src, Lang::Rust), vec!["a".to_string()]);
    }

    #[test]
    fn ts_only_exported_fns() {
        let src = "export function a() {}\nfunction b() {}\nexport const c = () => {};\nexport class D {}\nexport const e = 1;\n";
        assert_eq!(
            syms(src, Lang::TypeScript),
            vec!["a".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn python_only_nonunderscore_defs() {
        let src = "def a():\n    pass\ndef _b():\n    pass\nclass C:\n    pass\n";
        assert_eq!(syms(src, Lang::Python), vec!["a".to_string()]);
    }

    #[test]
    fn go_only_capitalized_funcs() {
        let src = "package p\nfunc Exported() {}\nfunc unexported() {}\ntype Foo struct{}\n";
        assert_eq!(syms(src, Lang::Go), vec!["Exported".to_string()]);
    }

    #[test]
    fn python_symbols_include_class_methods() {
        let src = "\
def top():
    pass
class C:
    def visible(self):
        pass
    async def visible(self):  # sync/async mirror dedupes
        pass
    def _private(self):
        pass
    def __init__(self):
        pass
class _Hidden:
    def m(self):
        pass
";
        assert_eq!(
            public_symbols(src, Lang::Python, Surface::Callables),
            vec![
                vec!["C".to_string(), "visible".to_string()],
                vec!["top".to_string()],
            ]
        );
    }

    #[test]
    fn python_decorated_methods_are_enumerated() {
        let src = "\
class C:
    @property
    def value(self):
        pass
";
        assert_eq!(
            public_symbols(src, Lang::Python, Surface::Callables),
            vec![vec!["C".to_string(), "value".to_string()]]
        );
    }

    #[test]
    fn go_symbols_include_methods_by_receiver() {
        // Pointer and value receivers both yield the bare type name; lowercase method and
        // method on an unexported type are excluded.
        let src = "\
package p
func Top() {}
type Builder struct{}
func (b *Builder) Set() {}
func (ls Labels) String() string { return \"\" }
func (b *Builder) internal() {}
type priv struct{}
func (p *priv) Exported() {}
";
        assert_eq!(
            public_symbols(src, Lang::Go, Surface::Callables),
            vec![
                vec!["Builder".to_string(), "Set".to_string()],
                vec!["Labels".to_string(), "String".to_string()],
                vec!["Top".to_string()],
            ]
        );
    }

    #[test]
    fn rust_symbols_are_top_level_fns_only() {
        let src = "pub fn a() {}\nfn b() {}\nimpl S { pub fn m(&self) {} }\n";
        assert_eq!(public_symbols(src, Lang::Rust, Surface::Callables), vec![vec!["a".to_string()]]);
    }

    #[test]
    fn python_all_adds_classes_and_non_callables() {
        // Every kind #28 made anchorable, plus the class itself — withheld under Callables,
        // proposed under All. `_`-prefixed names stay private in both modes (#52).
        let src = "\
CONST_VALUE = 42
MyAlias = dict[str, int]
type NewAlias = int
_hidden = 1

def top_level_func(x):
    return x

class TopLevelClass:
    attr: int = 1
    _secret = 2
    def method(self):
        return self.attr
";
        // Callables: only the function and the method.
        assert_eq!(
            public_symbols(src, Lang::Python, Surface::Callables),
            vec![
                vec!["TopLevelClass".to_string(), "method".to_string()],
                vec!["top_level_func".to_string()],
            ]
        );
        // All: also the class, both module-level aliases, the constant, and the class attribute.
        assert_eq!(
            public_symbols(src, Lang::Python, Surface::All),
            vec![
                vec!["CONST_VALUE".to_string()],
                vec!["MyAlias".to_string()],
                vec!["NewAlias".to_string()],
                vec!["TopLevelClass".to_string()],
                vec!["TopLevelClass".to_string(), "attr".to_string()],
                vec!["TopLevelClass".to_string(), "method".to_string()],
                vec!["top_level_func".to_string()],
            ]
        );
    }

    #[test]
    fn all_is_callables_only_for_non_python() {
        // The non-callable extension is Python-scoped for now; --all must not change Rust/Go/TS
        // output, so a Rust repo's lint nudge and suggest stay identical across modes.
        let rust = "pub fn a() {}\npub struct S;\npub const K: u8 = 1;\n";
        assert_eq!(
            public_symbols(rust, Lang::Rust, Surface::All),
            public_symbols(rust, Lang::Rust, Surface::Callables)
        );
        let go = "package p\nfunc Top() {}\ntype Builder struct{}\nconst K = 1\n";
        assert_eq!(
            public_symbols(go, Lang::Go, Surface::All),
            public_symbols(go, Lang::Go, Surface::Callables)
        );
    }
}
