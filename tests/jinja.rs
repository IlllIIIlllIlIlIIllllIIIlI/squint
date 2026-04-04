mod common;
use common::{check, fix};
use squint::rules::jinja::JinjaPadding;

// ── JJ01 — Jinja padding ─────────────────────────────────────────────────────

#[test]
fn jj01_clean_expression() {
    assert!(check(JinjaPadding, "select {{ my_col }} from t").is_empty());
}

#[test]
fn jj01_clean_block() {
    assert!(check(JinjaPadding, "{% if condition %}select 1{% endif %}").is_empty());
}

#[test]
fn jj01_no_space_expression() {
    let v = check(JinjaPadding, "select {{my_col}} from t");
    assert!(!v.is_empty());
    assert_eq!(v[0].rule_id, "JJ01");
}

#[test]
fn jj01_no_space_block_start() {
    let v = check(JinjaPadding, "{%if condition%}select 1{%endif%}");
    assert!(!v.is_empty());
    assert_eq!(v[0].rule_id, "JJ01");
}

#[test]
fn jj01_not_fixable() {
    let sql = "select {{my_col}} from t";
    assert_eq!(fix(JinjaPadding, sql), sql);
}
