/// Integration tests for RF02 (wildcard column references).
mod common;
use common::check;
use squint::rules::references::WildcardColumns;

// ── RF02 — SELECT * ──────────────────────────────────────────────────────────

#[test]
fn rf02_explicit_columns_ok() {
    assert!(check(WildcardColumns, "select a, b, c from t\n").is_empty());
}

#[test]
fn rf02_bare_star_flagged() {
    let v = check(WildcardColumns, "select * from t\n");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "RF02");
}

#[test]
fn rf02_qualified_star_flagged() {
    let v = check(WildcardColumns, "select t.* from t\n");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "RF02");
}

#[test]
fn rf02_count_star_ok() {
    assert!(check(WildcardColumns, "select count(*) from t\n").is_empty());
}

#[test]
fn rf02_count_distinct_star_ok() {
    // count(distinct *) is unusual but the * is still inside parens.
    assert!(check(WildcardColumns, "select count(distinct *) from t\n").is_empty());
}

#[test]
fn rf02_star_with_other_columns_flagged() {
    let v = check(WildcardColumns, "select a, * from t\n");
    assert_eq!(v.len(), 1);
}

#[test]
fn rf02_arithmetic_multiplication_not_flagged() {
    assert!(check(WildcardColumns, "select a * b from t\n").is_empty());
}

#[test]
fn rf02_arithmetic_in_expression_not_flagged() {
    assert!(check(WildcardColumns, "select (a + b) * c from t\n").is_empty());
}

#[test]
fn rf02_multiplication_in_where_not_flagged() {
    assert!(check(WildcardColumns, "select a from t where b * 2 > 10\n").is_empty());
}

#[test]
fn rf02_cte_star_flagged() {
    let sql = "with cte as (\n    select * from t\n)\nselect id from cte\n";
    let v = check(WildcardColumns, sql);
    assert_eq!(v.len(), 1);
}

#[test]
fn rf02_subquery_star_flagged() {
    let sql = "select id from (select * from t) sub\n";
    let v = check(WildcardColumns, sql);
    assert_eq!(v.len(), 1);
}

#[test]
fn rf02_violation_position_reported() {
    let v = check(WildcardColumns, "select * from t\n");
    assert_eq!(v[0].line, 1);
    // "select " is 7 chars, star is at col 8.
    assert_eq!(v[0].col, 8);
}
