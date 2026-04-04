mod common;
use common::{check, fix};
use squint::rules::layout::{
    CteBracket, CteNewline, EndOfFile, FunctionSpacing, Indent, LineLength, SelectModifiers,
    SetOperators, Spacing,
};

// ── LT01 — Spacing ───────────────────────────────────────────────────────────

#[test]
fn lt01_clean() {
    assert!(check(Spacing, "select a, b from t").is_empty());
}

#[test]
fn lt01_space_before_comma() {
    let v = check(Spacing, "select a , b from t");
    assert!(!v.is_empty());
    assert_eq!(v[0].rule_id, "LT01");
    assert!(v[0].message.contains("before ','"));
}

#[test]
fn lt01_double_space_mid_line() {
    let v = check(Spacing, "select  a from t");
    assert!(!v.is_empty());
    assert_eq!(v[0].rule_id, "LT01");
}

#[test]
fn lt01_func_space() {
    let v = check(Spacing, "select count (id) from t");
    assert!(!v.is_empty());
    assert_eq!(v[0].rule_id, "LT01");
}

#[test]
fn lt01_indentation_not_flagged() {
    // Leading spaces on a new line are indentation — LT02's job, not LT01's.
    assert!(check(Spacing, "select\n    a\nfrom t").is_empty());
}

#[test]
fn lt01_fix_space_before_comma() {
    assert_eq!(fix(Spacing, "select a , b from t"), "select a, b from t");
}

#[test]
fn lt01_fix_double_space() {
    assert_eq!(fix(Spacing, "select  a from t"), "select a from t");
}

#[test]
fn lt01_fix_func_space() {
    assert_eq!(
        fix(Spacing, "select count (id) from t"),
        "select count(id) from t"
    );
}

// ── LT02 — Indentation ───────────────────────────────────────────────────────

#[test]
fn lt02_clean() {
    assert!(check(Indent, "select\n    id\nfrom t").is_empty());
}

#[test]
fn lt02_eight_spaces_ok() {
    assert!(check(Indent, "select\n        id\nfrom t").is_empty());
}

#[test]
fn lt02_tab_violation() {
    let v = check(Indent, "select\n\tid\nfrom t");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "LT02");
    assert!(v[0].message.contains("tabs"));
}

#[test]
fn lt02_odd_indent() {
    let v = check(Indent, "select\n   id\nfrom t");
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("multiple of 4"));
}

#[test]
fn lt02_violation_reported_at_start_of_line() {
    let v = check(Indent, "select\n\tid\nfrom t");
    assert_eq!(v[0].line, 2);
    assert_eq!(v[0].col, 1);
}

// ── LT05 — Line length ───────────────────────────────────────────────────────

#[test]
fn lt05_clean() {
    assert!(check(LineLength::new(120), "select a from t").is_empty());
}

#[test]
fn lt05_exact_limit_ok() {
    assert!(check(LineLength::new(120), &"x".repeat(120)).is_empty());
}

#[test]
fn lt05_over_limit() {
    let v = check(LineLength::new(120), &"x".repeat(121));
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "LT05");
    assert_eq!(v[0].col, 121);
}

#[test]
fn lt05_custom_limit() {
    let v = check(
        LineLength::new(40),
        "select a_very_long_column_name from a_very_long_table_name",
    );
    assert_eq!(v.len(), 1);
}

#[test]
fn lt05_only_long_line_flagged() {
    let sql = "select a from t\nselect a_very_long_column_name from a_very_long_table_name\nselect b from t";
    let v = check(LineLength::new(40), sql);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].line, 2);
}

// ── LT06 — Function spacing ───────────────────────────────────────────────────

#[test]
fn lt06_clean() {
    assert!(check(FunctionSpacing, "select count(id) from t").is_empty());
}

#[test]
fn lt06_space_violation() {
    let v = check(FunctionSpacing, "select count (id) from t");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "LT06");
    assert!(v[0].message.contains("count"));
}

#[test]
fn lt06_fix() {
    assert_eq!(
        fix(FunctionSpacing, "select count (id) from t"),
        "select count(id) from t"
    );
}

#[test]
fn lt06_fix_multiple_functions() {
    assert_eq!(
        fix(FunctionSpacing, "select sum (a), max (b) from t"),
        "select sum(a), max(b) from t"
    );
}

// ── LT07 — CTE bracket on own line ───────────────────────────────────────────

#[test]
fn lt07_clean() {
    assert!(check(
        CteBracket,
        "with cte as (\n    select 1\n)\nselect * from cte"
    )
    .is_empty());
}

#[test]
fn lt07_violation_inline() {
    let v = check(CteBracket, "with cte as (select 1)\nselect * from cte");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "LT07");
    assert!(v[0].message.contains("cte"));
}

// ── LT08 — Blank line after CTE ──────────────────────────────────────────────

#[test]
fn lt08_clean() {
    assert!(check(
        CteNewline,
        "with cte as (\n    select 1\n)\n\nselect * from cte"
    )
    .is_empty());
}

#[test]
fn lt08_missing_blank_line() {
    let v = check(
        CteNewline,
        "with cte as (\n    select 1\n)\nselect * from cte",
    );
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "LT08");
    assert!(v[0].message.contains("cte"));
}

// ── LT10 — SELECT modifier on same line ──────────────────────────────────────

#[test]
fn lt10_clean() {
    assert!(check(SelectModifiers, "select distinct a from t").is_empty());
}

#[test]
fn lt10_modifier_on_next_line() {
    let v = check(SelectModifiers, "select\ndistinct a from t");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "LT10");
    assert!(v[0].message.contains("DISTINCT"));
}

// ── LT11 — Set operators on own line ─────────────────────────────────────────

#[test]
fn lt11_clean() {
    assert!(check(SetOperators, "select a from t\nunion all\nselect a from u").is_empty());
}

#[test]
fn lt11_inline_union() {
    let v = check(SetOperators, "select a from t union all select a from u");
    assert!(!v.is_empty());
    assert_eq!(v[0].rule_id, "LT11");
}

// ── LT12 — Trailing newline ───────────────────────────────────────────────────

#[test]
fn lt12_clean() {
    assert!(check(EndOfFile, "select 1\n").is_empty());
}

#[test]
fn lt12_empty_file_ok() {
    assert!(check(EndOfFile, "").is_empty());
}

#[test]
fn lt12_missing_newline() {
    let v = check(EndOfFile, "select 1");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "LT12");
}

#[test]
fn lt12_two_trailing_newlines() {
    let v = check(EndOfFile, "select 1\n\n");
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("2 trailing"));
}

#[test]
fn lt12_three_trailing_newlines() {
    let v = check(EndOfFile, "select 1\n\n\n");
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("3 trailing"));
}

#[test]
fn lt12_fix_missing_newline() {
    assert_eq!(fix(EndOfFile, "select 1"), "select 1\n");
}

#[test]
fn lt12_fix_extra_newlines() {
    assert_eq!(fix(EndOfFile, "select 1\n\n\n"), "select 1\n");
}

#[test]
fn lt12_fix_idempotent() {
    let sql = "select 1\n";
    assert_eq!(fix(EndOfFile, sql), sql);
}
