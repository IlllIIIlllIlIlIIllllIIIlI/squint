mod common;
use common::check;
use squint::config::GroupByStyle;
use squint::rules::ambiguous::{ColumnReferences, DistinctGroupBy, UnionDistinct};

// ── AM01 — DISTINCT + GROUP BY ────────────────────────────────────────────────

#[test]
fn am01_clean_distinct_only() {
    assert!(check(DistinctGroupBy, "select distinct a from t").is_empty());
}

#[test]
fn am01_clean_group_by_only() {
    assert!(check(DistinctGroupBy, "select a from t group by a").is_empty());
}

#[test]
fn am01_distinct_with_group_by() {
    let v = check(DistinctGroupBy, "select distinct a from t group by a");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "AM01");
    assert!(v[0].message.contains("redundant"));
}

// ── AM02 — UNION must have qualifier ─────────────────────────────────────────

#[test]
fn am02_clean_union_all() {
    assert!(check(UnionDistinct, "select a from t union all select b from t").is_empty());
}

#[test]
fn am02_clean_union_distinct() {
    assert!(check(
        UnionDistinct,
        "select a from t union distinct select b from t"
    )
    .is_empty());
}

#[test]
fn am02_bare_union() {
    let v = check(UnionDistinct, "select a from t union select b from t");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "AM02");
    assert!(v[0].message.contains("DISTINCT or ALL"));
}

// ── AM06 — GROUP BY / ORDER BY style ─────────────────────────────────────────

#[test]
fn am06_explicit_clean() {
    assert!(check(
        ColumnReferences::new(GroupByStyle::Explicit),
        "select a, b from t group by a, b"
    )
    .is_empty());
}

#[test]
fn am06_explicit_rejects_positional() {
    let v = check(
        ColumnReferences::new(GroupByStyle::Explicit),
        "select a, b from t group by 1, 2",
    );
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].rule_id, "AM06");
}

#[test]
fn am06_implicit_clean() {
    assert!(check(
        ColumnReferences::new(GroupByStyle::Implicit),
        "select a, b from t order by 1, 2"
    )
    .is_empty());
}

#[test]
fn am06_implicit_rejects_explicit() {
    let v = check(
        ColumnReferences::new(GroupByStyle::Implicit),
        "select a, b from t order by a, b",
    );
    assert_eq!(v.len(), 2);
}

#[test]
fn am06_consistent_pure_explicit_ok() {
    assert!(check(
        ColumnReferences::new(GroupByStyle::Consistent),
        "select a, b from t group by a, b"
    )
    .is_empty());
}

#[test]
fn am06_consistent_mixed() {
    let v = check(
        ColumnReferences::new(GroupByStyle::Consistent),
        "select a, b from t group by a, 2",
    );
    assert!(!v.is_empty());
    assert_eq!(v[0].rule_id, "AM06");
}
