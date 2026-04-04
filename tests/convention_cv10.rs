/// Integration tests for CV10 (consistent quoting style).
mod common;
use common::check;
use squint::rules::convention::QuotingStyle;

// ── CV10 — consistent quoting style ──────────────────────────────────────────

#[test]
fn cv10_all_unquoted_ok() {
    assert!(check(QuotingStyle, "select col from my_table\n").is_empty());
}

#[test]
fn cv10_all_quoted_ok() {
    assert!(check(
        QuotingStyle,
        "select \"col\" from \"my_table\"\n"
    )
    .is_empty());
}

#[test]
fn cv10_different_names_each_consistent_ok() {
    // 'a' always unquoted, 'b' always quoted — no mixing
    assert!(check(QuotingStyle, "select a, \"b\" from t\n").is_empty());
}

#[test]
fn cv10_unquoted_then_quoted_flagged() {
    let v = check(QuotingStyle, "select col, \"col\" from t\n");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "CV10");
    assert!(v[0].message.contains("col"));
}

#[test]
fn cv10_quoted_then_unquoted_flagged() {
    let v = check(QuotingStyle, "select \"col\", col from t\n");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "CV10");
}

#[test]
fn cv10_only_first_conflict_per_name() {
    // 'col' unquoted once, then quoted twice — only one violation
    let v = check(QuotingStyle, "select col, \"col\", \"col\" from t\n");
    assert_eq!(v.len(), 1);
}

#[test]
fn cv10_multiple_inconsistent_names_flagged() {
    let v = check(
        QuotingStyle,
        "select a, \"b\", b, \"a\" from t\n",
    );
    assert_eq!(v.len(), 2); // both 'a' and 'b' are inconsistent
}

#[test]
fn cv10_backtick_quoting_flagged() {
    let v = check(QuotingStyle, "select col, `col` from t\n");
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("col"));
}

#[test]
fn cv10_case_insensitive_comparison() {
    // Unquoted COL and quoted "col" are treated as the same identifier
    let v = check(QuotingStyle, "select COL, \"col\" from t\n");
    assert_eq!(v.len(), 1);
}

#[test]
fn cv10_violation_position_reported() {
    // "select col, " is 12 chars; "col" starts at byte 12 (col 13)
    let v = check(QuotingStyle, "select col, \"col\" from t\n");
    assert_eq!(v[0].line, 1);
    assert_eq!(v[0].col, 13);
}

#[test]
fn cv10_table_name_inconsistency_flagged() {
    let v = check(
        QuotingStyle,
        "select a from my_table t join \"my_table\" u on t.id = u.id\n",
    );
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("my_table"));
}
