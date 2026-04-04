mod common;
use common::{check, fix};
use squint::rules::aliasing::{
    AliasLength, ExpressionAliases, ImplicitAliases, SelfAliases, UniqueColumnAliases,
    UniqueTableAliases, UnusedAliases,
};

// ── AL02 — Implicit aliases ───────────────────────────────────────────────────

#[test]
fn al02_clean_explicit() {
    assert!(check(ImplicitAliases, "select a as b from t").is_empty());
}

#[test]
fn al02_clean_no_alias() {
    assert!(check(ImplicitAliases, "select a, b from t").is_empty());
}

#[test]
fn al02_implicit_alias() {
    let v = check(ImplicitAliases, "select a alias_col from t");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "AL02");
    assert!(v[0].message.contains("alias_col"));
}

#[test]
fn al02_not_fixable() {
    let sql = "select a alias_col from t";
    assert_eq!(fix(ImplicitAliases, sql), sql);
}

// ── AL03 — Expression aliases ─────────────────────────────────────────────────

#[test]
fn al03_clean_bare_col() {
    assert!(check(ExpressionAliases, "select a, b from t").is_empty());
}

#[test]
fn al03_clean_aliased_function() {
    assert!(check(ExpressionAliases, "select count(id) as cnt from t").is_empty());
}

#[test]
fn al03_unaliased_function() {
    let v = check(ExpressionAliases, "select count(id) from t");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "AL03");
}

#[test]
fn al03_unaliased_arithmetic() {
    let v = check(ExpressionAliases, "select a + b from t");
    assert_eq!(v.len(), 1);
}

#[test]
fn al03_not_fixable() {
    let sql = "select count(id) from t";
    assert_eq!(fix(ExpressionAliases, sql), sql);
}

// ── AL04 — Unique table aliases ───────────────────────────────────────────────

#[test]
fn al04_clean() {
    assert!(check(
        UniqueTableAliases,
        "select * from foo as a join bar as b on a.id = b.id"
    )
    .is_empty());
}

#[test]
fn al04_duplicate_alias() {
    let v = check(
        UniqueTableAliases,
        "select * from foo as t join bar as t on t.id = t.id",
    );
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "AL04");
    assert!(v[0].message.contains("'t'"));
}

// ── AL05 — Unused aliases ─────────────────────────────────────────────────────

#[test]
fn al05_clean() {
    assert!(check(UnusedAliases, "select a.id from foo as a").is_empty());
}

#[test]
fn al05_unused() {
    let v = check(UnusedAliases, "select id from foo as unused_alias");
    assert!(!v.is_empty());
    assert_eq!(v[0].rule_id, "AL05");
    assert!(v[0].message.contains("unused_alias"));
}

// ── AL06 — Alias length ───────────────────────────────────────────────────────

#[test]
fn al06_clean() {
    assert!(check(AliasLength::new(1, 0), "select * from foo as ab").is_empty());
}

#[test]
fn al06_no_max() {
    assert!(check(
        AliasLength::new(1, 0),
        "select * from foo as very_long_alias_name"
    )
    .is_empty());
}

#[test]
fn al06_too_short() {
    let v = check(AliasLength::new(2, 0), "select * from foo as a");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "AL06");
    assert!(v[0].message.contains("too short"));
}

#[test]
fn al06_too_long() {
    let v = check(
        AliasLength::new(1, 5),
        "select * from foo as very_long_alias",
    );
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("too long"));
}

// ── AL08 — Unique column aliases ──────────────────────────────────────────────

#[test]
fn al08_clean() {
    assert!(check(UniqueColumnAliases, "select a as x, b as y from t").is_empty());
}

#[test]
fn al08_duplicate() {
    let v = check(UniqueColumnAliases, "select a as foo, b as foo from t");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "AL08");
}

#[test]
fn al08_case_insensitive() {
    let v = check(UniqueColumnAliases, "select a as foo, b as FOO from t");
    assert_eq!(v.len(), 1);
}

// ── AL09 — Self aliases ───────────────────────────────────────────────────────

#[test]
fn al09_clean() {
    assert!(check(SelfAliases, "select a as b from t").is_empty());
}

#[test]
fn al09_self_alias() {
    let v = check(SelfAliases, "select col as col from t");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "AL09");
    assert!(v[0].message.contains("col"));
}

#[test]
fn al09_case_insensitive_self_alias() {
    let v = check(SelfAliases, "select Col as col from t");
    assert_eq!(v.len(), 1);
}
