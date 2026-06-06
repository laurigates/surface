use surf_core::{parse_anchor, resolve, Lang, ResolveError, Span};

const TS: &str = include_str!("fixtures/auth.ts");
const RS: &str = include_str!("fixtures/auth.rs");

fn span(src: &str, lang: Lang, anchor: &str) -> Span {
    let a = parse_anchor(anchor).unwrap();
    resolve(src, lang, &a).unwrap_or_else(|e| panic!("resolve `{anchor}` failed: {e}"))
}

fn snippet(src: &str, s: Span) -> &str {
    &src[s.start_byte..s.end_byte]
}

fn err(src: &str, lang: Lang, anchor: &str) -> ResolveError {
    let a = parse_anchor(anchor).unwrap();
    resolve(src, lang, &a).expect_err("expected resolution to fail")
}

// --- TypeScript -----------------------------------------------------------

#[test]
fn ts_method_disambiguated_by_class() {
    // Both classes declare `rotate`; the qualified path picks the right one.
    let a = span(TS, Lang::TypeScript, "auth.ts > TokenService > rotate");
    assert!(snippet(TS, a).contains("token + \"!\""));

    let b = span(TS, Lang::TypeScript, "auth.ts > OtherService > rotate");
    assert!(snippet(TS, b).contains("token + \"?\""));

    assert_ne!((a.start_byte, a.end_byte), (b.start_byte, b.end_byte));
}

#[test]
fn ts_overloads_are_ambiguous_then_positional() {
    // Two signatures + one implementation, all named `rotate`, at top level.
    match err(TS, Lang::TypeScript, "auth.ts > rotate") {
        ResolveError::Ambiguous { count, .. } => assert_eq!(count, 3),
        other => panic!("expected Ambiguous, got {other:?}"),
    }
    // @3 selects the implementation (the one with a body).
    let impl_fn = span(TS, Lang::TypeScript, "auth.ts > rotate @3");
    assert!(snippet(TS, impl_fn).contains("force ? token.toUpperCase()"));
}

#[test]
fn ts_nested_function_in_arrow_const() {
    let s = span(TS, Lang::TypeScript, "auth.ts > refresh > inner");
    assert!(snippet(TS, s).contains("t.trim()"));
}

#[test]
fn ts_not_found_is_distinct() {
    match err(TS, Lang::TypeScript, "auth.ts > doesNotExist") {
        ResolveError::NotFound { segment } => assert_eq!(segment, "doesNotExist"),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

// --- Rust -----------------------------------------------------------------

#[test]
fn rust_method_via_impl_scope() {
    // `TokenService` alone matches both the struct and its impl, but the path
    // `TokenService > rotate` resolves uniquely to the method.
    let s = span(RS, Lang::Rust, "auth.rs > TokenService > rotate");
    assert!(snippet(RS, s).contains("helper(token)"));
}

#[test]
fn rust_top_level_fn_is_scoped() {
    // A free `rotate`, a method `rotate`, and a module fn `rotate` all exist;
    // the top-level path resolves to exactly the free function.
    let s = span(RS, Lang::Rust, "auth.rs > rotate");
    assert!(snippet(RS, s).starts_with("pub fn rotate(token: &str)"));
}

#[test]
fn rust_module_function() {
    let s = span(RS, Lang::Rust, "auth.rs > refresh > rotate");
    assert!(snippet(RS, s).contains("token.to_string()"));
    // Distinct from the free function and the method of the same name.
    let free = span(RS, Lang::Rust, "auth.rs > rotate");
    assert_ne!((s.start_byte, s.end_byte), (free.start_byte, free.end_byte));
}

#[test]
fn rust_deeply_nested_function() {
    let s = span(RS, Lang::Rust, "auth.rs > refresh > nested > deep");
    assert!(snippet(RS, s).contains("7"));
}

#[test]
fn rust_type_alone_is_ambiguous() {
    // struct + impl share the name.
    match err(RS, Lang::Rust, "auth.rs > TokenService") {
        ResolveError::Ambiguous { count, .. } => assert_eq!(count, 2),
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

#[test]
fn span_lines_are_one_based() {
    let s = span(RS, Lang::Rust, "auth.rs > TokenService > validate");
    assert!(s.start_line >= 1 && s.end_line >= s.start_line);
}
