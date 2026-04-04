mod common;
use common::check;
use squint::rules::structure::{Distinct, UnusedCte};

// ── ST03 — Unused CTE ────────────────────────────────────────────────────────

#[test]
fn st03_clean() {
    assert!(check(
        UnusedCte,
        "with cte as (\n    select 1\n)\nselect * from cte"
    )
    .is_empty());
}

#[test]
fn st03_unused_cte() {
    let v = check(UnusedCte, "with cte as (\n    select 1\n)\nselect 1");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "ST03");
    assert!(v[0].message.contains("cte"));
}

// ── ST08 — DISTINCT on constants ─────────────────────────────────────────────

#[test]
fn st08_clean() {
    assert!(check(Distinct, "select distinct a from t").is_empty());
}

#[test]
fn st08_count_distinct_star() {
    let v = check(Distinct, "select count(distinct *) from t");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "ST08");
    assert!(v[0].message.contains("COUNT(DISTINCT *)"));
}

#[test]
fn st08_distinct_number_literal() {
    let v = check(Distinct, "select distinct 1");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "ST08");
}

#[test]
fn st08_distinct_string_literal() {
    let v = check(Distinct, "select distinct 'hello'");
    assert_eq!(v.len(), 1);
}
