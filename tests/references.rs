mod common;
use common::check;
use squint::rules::references::References;

// ── RF01 — Column qualifier references ───────────────────────────────────────

#[test]
fn rf01_clean_with_alias() {
    assert!(check(References, "select a.id from foo as a").is_empty());
}

#[test]
fn rf01_clean_bare_table_name() {
    assert!(check(References, "select foo.id from foo").is_empty());
}

#[test]
fn rf01_unknown_qualifier() {
    let v = check(References, "select x.id from foo as a");
    assert!(!v.is_empty());
    assert_eq!(v[0].rule_id, "RF01");
    assert!(v[0].message.contains("x"));
}
