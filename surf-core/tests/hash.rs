use surf_core::{
    diff_magnitude, hash_anchor, hash_anchor_with, parse_anchor, HashOpts, Lang, Magnitude,
};

fn h(src: &str, lang: Lang, anchor: &str) -> String {
    let a = parse_anchor(anchor).unwrap();
    hash_anchor(src, lang, &a).unwrap_or_else(|e| panic!("hash `{anchor}` failed: {e}"))
}

fn h_opts(src: &str, lang: Lang, anchor: &str, opts: HashOpts) -> String {
    let a = parse_anchor(anchor).unwrap();
    hash_anchor_with(src, lang, &a, opts).unwrap_or_else(|e| panic!("hash `{anchor}` failed: {e}"))
}

fn mag(old: &str, new: &str, lang: Lang, anchor: &str) -> Magnitude {
    let a = parse_anchor(anchor).unwrap();
    diff_magnitude(old, new, lang, &a).unwrap()
}

// --- TypeScript: quiet on reformat / comments / rename ---------------------

const TS_BASE: &str = r#"
function add(a: number, b: number): number {
  return a + b;
}
"#;

#[test]
fn ts_quiet_on_reformatting_and_comments() {
    let reformatted = r#"
function add(a: number,     b: number): number {


    // add the two operands together
    return a   +   b; // trailing comment
}
"#;
    assert_eq!(
        h(TS_BASE, Lang::TypeScript, "f.ts > add"),
        h(reformatted, Lang::TypeScript, "f.ts > add")
    );
}

#[test]
fn ts_quiet_on_consistent_rename() {
    // Parameters a,b -> x,y consistently; structure identical.
    let renamed = r#"
function add(x: number, y: number): number {
  return x + y;
}
"#;
    assert_eq!(
        h(TS_BASE, Lang::TypeScript, "f.ts > add"),
        h(renamed, Lang::TypeScript, "f.ts > add")
    );
}

// --- TypeScript: loud on real changes --------------------------------------

#[test]
fn ts_loud_on_operator_flip() {
    let flipped = r#"
function add(a: number, b: number): number {
  return a - b;
}
"#;
    assert_ne!(
        h(TS_BASE, Lang::TypeScript, "f.ts > add"),
        h(flipped, Lang::TypeScript, "f.ts > add")
    );
}

#[test]
fn ts_loud_on_swapped_operands() {
    // Not a rename: the binding pattern changes (a,b used in swapped positions).
    let swapped = r#"
function add(a: number, b: number): number {
  return b + a;
}
"#;
    assert_ne!(
        h(TS_BASE, Lang::TypeScript, "f.ts > add"),
        h(swapped, Lang::TypeScript, "f.ts > add")
    );
}

#[test]
fn ts_loud_on_relaxed_comparison() {
    let lt = "function gate(n: number): boolean { return n < 10; }";
    let lte = "function gate(n: number): boolean { return n <= 10; }";
    assert_ne!(
        h(lt, Lang::TypeScript, "f.ts > gate"),
        h(lte, Lang::TypeScript, "f.ts > gate")
    );
}

#[test]
fn ts_loud_on_deleted_await() {
    let with = "async function load(): Promise<number> { return await fetchNumber(); }";
    let without = "async function load(): Promise<number> { return fetchNumber(); }";
    assert_ne!(
        h(with, Lang::TypeScript, "f.ts > load"),
        h(without, Lang::TypeScript, "f.ts > load")
    );
}

#[test]
fn ts_loud_on_changed_constant() {
    let one = "function timeout(): number { return 30; }";
    let two = "function timeout(): number { return 60; }";
    assert_ne!(
        h(one, Lang::TypeScript, "f.ts > timeout"),
        h(two, Lang::TypeScript, "f.ts > timeout")
    );
}

// --- Rust: same properties -------------------------------------------------

const RS_BASE: &str = r#"
fn add(a: i64, b: i64) -> i64 {
    a + b
}
"#;

#[test]
fn rust_quiet_on_reformat_and_rename() {
    let variant = r#"
fn add(x: i64, y: i64) -> i64 {
    // commutative sum
    x  +  y
}
"#;
    assert_eq!(
        h(RS_BASE, Lang::Rust, "f.rs > add"),
        h(variant, Lang::Rust, "f.rs > add")
    );
}

#[test]
fn rust_loud_on_operator_flip() {
    let flipped = "fn add(a: i64, b: i64) -> i64 { a - b }";
    assert_ne!(
        h(RS_BASE, Lang::Rust, "f.rs > add"),
        h(flipped, Lang::Rust, "f.rs > add")
    );
}

#[test]
fn rust_loud_on_changed_primitive_type() {
    let i64 = "fn id(a: i64) -> i64 { a }";
    let i32 = "fn id(a: i32) -> i32 { a }";
    assert_ne!(
        h(i64, Lang::Rust, "f.rs > id"),
        h(i32, Lang::Rust, "f.rs > id")
    );
}

// --- Python ----------------------------------------------------------------

const PY_BASE: &str = "def add(a, b):\n    return a + b\n";

#[test]
fn python_quiet_on_reformat_comment_and_rename() {
    let variant = "def add(x,   y):\n    # sum\n    return x  +  y\n";
    assert_eq!(
        h(PY_BASE, Lang::Python, "f.py > add"),
        h(variant, Lang::Python, "f.py > add")
    );
}

#[test]
fn python_loud_on_operator_flip() {
    let flipped = "def add(a, b):\n    return a - b\n";
    assert_ne!(
        h(PY_BASE, Lang::Python, "f.py > add"),
        h(flipped, Lang::Python, "f.py > add")
    );
}

// --- Python: decorators are part of the hashed span (#8) -------------------

#[test]
fn python_loud_on_decorator_change() {
    // In tree-sitter-python the decorator lives in the parent `decorated_definition`; the hash
    // widens to it, so swapping or parameterizing a decorator changes the hash.
    let base = "@cache\ndef f(x):\n    return x + 1\n";
    let renamed_decorator = "@lru_cache\ndef f(x):\n    return x + 1\n";
    let parameterized = "@lru_cache(maxsize=128)\ndef f(x):\n    return x + 1\n";
    assert_ne!(
        h(base, Lang::Python, "f.py > f"),
        h(renamed_decorator, Lang::Python, "f.py > f")
    );
    assert_ne!(
        h(renamed_decorator, Lang::Python, "f.py > f"),
        h(parameterized, Lang::Python, "f.py > f")
    );
}

#[test]
fn python_decorated_still_quiet_on_reformat() {
    let base = "@cache\ndef f(x):\n    return x + 1\n";
    let reformatted = "@cache\ndef f(y):\n    # bump\n    return y  +  1\n";
    assert_eq!(
        h(base, Lang::Python, "f.py > f"),
        h(reformatted, Lang::Python, "f.py > f")
    );
}

// --- Per-claim ignore_literals (#21) ---------------------------------------

#[test]
fn ignore_literals_quiet_on_string_content() {
    let ignore = HashOpts {
        ignore_literals: true,
    };
    let base = "def notify(): return \"Nominate someone!\"\n";
    let copy_edit = "def notify(): return \"Go nominate someone!\"\n";
    // Default: a copy edit trips the gate.
    assert_ne!(
        h(base, Lang::Python, "f.py > notify"),
        h(copy_edit, Lang::Python, "f.py > notify")
    );
    // ignore_literals: the copy edit is invisible.
    assert_eq!(
        h_opts(base, Lang::Python, "f.py > notify", ignore),
        h_opts(copy_edit, Lang::Python, "f.py > notify", ignore)
    );
}

#[test]
fn ignore_literals_still_loud_on_logic() {
    let ignore = HashOpts {
        ignore_literals: true,
    };
    // Same string, but the comparison operator changes — still caught even when ignoring copy.
    let base = "function gate(n: number): string { return n < 10 ? \"ok\" : \"no\"; }";
    let logic = "function gate(n: number): string { return n <= 10 ? \"ok\" : \"no\"; }";
    assert_ne!(
        h_opts(base, Lang::TypeScript, "f.ts > gate", ignore),
        h_opts(logic, Lang::TypeScript, "f.ts > gate", ignore)
    );
    // And a numeric constant change is still logic, not a literal we ignore.
    let num = "function gate(n: number): string { return n < 20 ? \"ok\" : \"no\"; }";
    assert_ne!(
        h_opts(base, Lang::TypeScript, "f.ts > gate", ignore),
        h_opts(num, Lang::TypeScript, "f.ts > gate", ignore)
    );
}

// --- Go --------------------------------------------------------------------

const GO_BASE: &str = "package p\nfunc add(a int, b int) int {\n\treturn a + b\n}\n";

#[test]
fn go_quiet_on_reformat_and_rename() {
    let variant = "package p\nfunc add(x int, y int) int {\n\t// sum\n\treturn x + y\n}\n";
    assert_eq!(
        h(GO_BASE, Lang::Go, "f.go > add"),
        h(variant, Lang::Go, "f.go > add")
    );
}

#[test]
fn go_loud_on_operator_flip() {
    let flipped = "package p\nfunc add(a int, b int) int {\n\treturn a - b\n}\n";
    assert_ne!(
        h(GO_BASE, Lang::Go, "f.go > add"),
        h(flipped, Lang::Go, "f.go > add")
    );
}

// --- JavaScript ------------------------------------------------------------

const JS_BASE: &str = "function add(a, b) {\n  return a + b;\n}\n";

#[test]
fn js_quiet_on_rename_loud_on_operator() {
    let renamed = "function add(x, y) {\n  return x + y;\n}\n";
    let flipped = "function add(a, b) {\n  return a - b;\n}\n";
    assert_eq!(
        h(JS_BASE, Lang::JavaScript, "f.js > add"),
        h(renamed, Lang::JavaScript, "f.js > add")
    );
    assert_ne!(
        h(JS_BASE, Lang::JavaScript, "f.js > add"),
        h(flipped, Lang::JavaScript, "f.js > add")
    );
}

// --- Magnitude is advisory and plausible -----------------------------------

#[test]
fn magnitude_small_for_operator_flip() {
    let flipped = "function add(a: number, b: number): number { return a - b; }";
    assert_eq!(
        mag(TS_BASE, flipped, Lang::TypeScript, "f.ts > add"),
        Magnitude::Small
    );
}

#[test]
fn magnitude_large_for_rewrite() {
    let rewrite = r#"
function add(a: number, b: number): number {
  const scale = a * 2;
  const offset = b - 1;
  if (scale > offset) {
    return scale + offset + a + b;
  }
  return scale - offset;
}
"#;
    assert_eq!(
        mag(TS_BASE, rewrite, Lang::TypeScript, "f.ts > add"),
        Magnitude::Large
    );
}

#[test]
fn magnitude_does_not_affect_hash_equality() {
    // A consistent rename has nonzero textual change but the hash is unchanged:
    // proof that the gate's verdict is the hash alone, and magnitude is orthogonal.
    let renamed = "function add(x: number, y: number): number { return x + y; }";
    assert_eq!(
        h(TS_BASE, Lang::TypeScript, "f.ts > add"),
        h(renamed, Lang::TypeScript, "f.ts > add")
    );
    assert_eq!(
        mag(TS_BASE, renamed, Lang::TypeScript, "f.ts > add"),
        Magnitude::Small
    );
}

// --- Python @overload groups hash as one unit (#82) -------------------------

const OVERLOADED_PY: &str = r#"
from typing import overload

@overload
def probe(x: int) -> int: ...
@overload
def probe(x: str) -> str: ...
def probe(x):
    return x
"#;

#[test]
fn python_overload_stub_signature_change_changes_hash() {
    // The gate hole from #82: the stubs *are* the public contract, so editing one must
    // change the group hash even though the implementation body is untouched.
    let stub_changed = r#"
from typing import overload

@overload
def probe(x: int, base: int = 10) -> float: ...
@overload
def probe(x: str) -> str: ...
def probe(x):
    return x
"#;
    assert_ne!(
        h(OVERLOADED_PY, Lang::Python, "f.py > probe"),
        h(stub_changed, Lang::Python, "f.py > probe")
    );
}

#[test]
fn python_overload_group_quiet_on_reformat_and_comments() {
    let reformatted = r#"
from typing import overload

@overload
def probe(x: int) -> int: ...

# the string overload
@overload
def probe(x: str) -> str: ...

def probe(x):
    return x  # identity
"#;
    assert_eq!(
        h(OVERLOADED_PY, Lang::Python, "f.py > probe"),
        h(reformatted, Lang::Python, "f.py > probe")
    );
}

#[test]
fn python_overload_impl_change_still_changes_hash() {
    let impl_changed = r#"
from typing import overload

@overload
def probe(x: int) -> int: ...
@overload
def probe(x: str) -> str: ...
def probe(x):
    return None
"#;
    assert_ne!(
        h(OVERLOADED_PY, Lang::Python, "f.py > probe"),
        h(impl_changed, Lang::Python, "f.py > probe")
    );
}
