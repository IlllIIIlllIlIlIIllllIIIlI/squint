mod common;
use common::{check, fix};
use squint::config::TrailingCommaPolicy;
use squint::rules::convention::{CountRows, IsNull, TrailingComma};

// ── CV03 — Trailing comma ─────────────────────────────────────────────────────

#[test]
fn cv03_forbid_clean() {
    assert!(check(
        TrailingComma::new(TrailingCommaPolicy::Forbid),
        "select a, b from t"
    )
    .is_empty());
}

#[test]
fn cv03_forbid_trailing_comma_violation() {
    let v = check(
        TrailingComma::new(TrailingCommaPolicy::Forbid),
        "select a, b, from t",
    );
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "CV03");
}

#[test]
fn cv03_require_clean() {
    assert!(check(
        TrailingComma::new(TrailingCommaPolicy::Require),
        "select a, b, from t"
    )
    .is_empty());
}

#[test]
fn cv03_require_missing_comma() {
    let v = check(
        TrailingComma::new(TrailingCommaPolicy::Require),
        "select a, b from t",
    );
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "CV03");
}

// ── CV04 — COUNT(*) vs COUNT(1) ───────────────────────────────────────────────

#[test]
fn cv04_prefer_star_clean() {
    assert!(check(CountRows::new(false), "select count(*) from t").is_empty());
}

#[test]
fn cv04_prefer_star_rejects_count_1() {
    let v = check(CountRows::new(false), "select count(1) from t");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "CV04");
    assert!(v[0].message.contains("COUNT(*)"));
}

#[test]
fn cv04_prefer_one_clean() {
    assert!(check(CountRows::new(true), "select count(1) from t").is_empty());
}

#[test]
fn cv04_prefer_one_rejects_star() {
    let v = check(CountRows::new(true), "select count(*) from t");
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("COUNT(1)"));
}

// ── CV05 — NULL comparisons ───────────────────────────────────────────────────

#[test]
fn cv05_clean_is_null() {
    assert!(check(IsNull, "select * from t where a is null").is_empty());
}

#[test]
fn cv05_clean_is_not_null() {
    assert!(check(IsNull, "select * from t where a is not null").is_empty());
}

#[test]
fn cv05_eq_null() {
    let v = check(IsNull, "select * from t where a = null");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "CV05");
    assert!(v[0].message.contains("IS NULL"));
}

#[test]
fn cv05_neq_null() {
    let v = check(IsNull, "select * from t where a != null");
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("IS NOT NULL"));
}

#[test]
fn cv05_diamond_null() {
    let v = check(IsNull, "select * from t where a <> null");
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("IS NOT NULL"));
}

#[test]
fn cv05_fix_eq_null() {
    assert_eq!(
        fix(IsNull, "select * from t where a = null"),
        "select * from t where a is null"
    );
}

#[test]
fn cv05_fix_neq_null() {
    assert_eq!(
        fix(IsNull, "select * from t where a != null"),
        "select * from t where a is not null"
    );
}

#[test]
fn cv05_fix_diamond_neq() {
    assert_eq!(
        fix(IsNull, "select * from t where a <> null"),
        "select * from t where a is not null"
    );
}

#[test]
fn cv05_fix_leaves_clean_source_unchanged() {
    let sql = "select * from t where a is null";
    assert_eq!(fix(IsNull, sql), sql);
}
