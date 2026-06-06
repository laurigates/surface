use surf_core::{diff_magnitude, hash_anchor, parse_anchor, Lang, Magnitude};

fn h(src: &str, lang: Lang, anchor: &str) -> String {
    let a = parse_anchor(anchor).unwrap();
    hash_anchor(src, lang, &a).unwrap_or_else(|e| panic!("hash `{anchor}` failed: {e}"))
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
