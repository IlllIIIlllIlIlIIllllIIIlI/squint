/// Integration tests for `-- noqa` per-line suppression.
///
/// Uses CP01 (KeywordCasing) as the probe rule — it has both check() and fixes(),
/// making it easy to verify both violation suppression and fix suppression.
/// A second rule (LT12) is used to verify targeted per-rule suppression.
mod common;

use squint::rules::capitalisation::KeywordCasing;
use squint::rules::layout::EndOfFile;
use squint::{linter::lint_source, rules::Violation};

fn check_cp01(sql: &str) -> Vec<Violation> {
    common::check(KeywordCasing, sql)
}

fn fix_cp01(sql: &str) -> String {
    common::fix(KeywordCasing, sql)
}

// ── bare `-- noqa` (suppress all) ────────────────────────────────────────────

#[test]
fn noqa_suppresses_all_rules_on_that_line() {
    // SELECT is uppercase — would normally be flagged by CP01.
    let sql = "SELECT a -- noqa\nFROM t";
    let v = check_cp01(sql);
    // FROM on line 2 is flagged; SELECT on line 1 is suppressed.
    assert_eq!(
        v.len(),
        1,
        "expected 1 violation (FROM on line 2), got: {:?}",
        v
    );
    assert_eq!(v[0].line, 2);
}

#[test]
fn noqa_does_not_affect_other_lines() {
    let sql = "SELECT a -- noqa\nFROM t\nWHERE x = 1";
    let v = check_cp01(sql);
    // FROM (line 2) and WHERE (line 3) are still flagged.
    let lines: Vec<usize> = v.iter().map(|v| v.line).collect();
    assert!(lines.contains(&2), "FROM on line 2 should be flagged");
    assert!(lines.contains(&3), "WHERE on line 3 should be flagged");
    assert!(
        !lines.contains(&1),
        "SELECT on line 1 should be suppressed by noqa"
    );
}

#[test]
fn noqa_suppresses_fix_on_that_line() {
    // `-- noqa` should also suppress auto-fixes on that line.
    let sql = "SELECT a -- noqa\nFROM t";
    let fixed = fix_cp01(sql);
    assert!(
        fixed.contains("SELECT a"),
        "SELECT should not be lowercased on noqa line"
    );
    assert!(
        fixed.contains("from t"),
        "FROM should be lowercased on non-noqa line"
    );
}

// ── `-- noqa: RULE_ID` (targeted suppression) ────────────────────────────────

#[test]
fn noqa_targeted_suppresses_listed_rule() {
    let sql = "SELECT a -- noqa: CP01\nFROM t";
    let v = check_cp01(sql);
    // SELECT on line 1 is suppressed for CP01 specifically.
    assert_eq!(
        v.len(),
        1,
        "expected 1 violation (FROM on line 2), got: {:?}",
        v
    );
    assert_eq!(v[0].line, 2);
}

#[test]
fn noqa_targeted_does_not_suppress_unlisted_rule() {
    // noqa only suppresses LT12 — CP01 violations should still appear.
    let sql = "SELECT a -- noqa: LT12\nFROM t";
    let v = check_cp01(sql);
    // SELECT on line 1 is NOT suppressed (different rule).
    let lines: Vec<usize> = v.iter().map(|v| v.line).collect();
    assert!(
        lines.contains(&1),
        "SELECT on line 1 should still be flagged (noqa targets LT12, not CP01)"
    );
}

#[test]
fn noqa_targeted_multiple_rules() {
    // Multi-rule noqa: both rules suppressed on that line.
    let cp01 = Box::new(KeywordCasing) as Box<dyn squint::rules::Rule>;
    let lt12 = Box::new(EndOfFile) as Box<dyn squint::rules::Rule>;
    let rule_refs: Vec<&dyn squint::rules::Rule> = vec![cp01.as_ref(), lt12.as_ref()];

    // The file has no trailing newline (LT12) and uppercase SELECT (CP01) on line 1.
    let sql = "SELECT a -- noqa: CP01, LT12";
    let violations = lint_source(sql, &rule_refs);
    assert!(
        violations.is_empty(),
        "both CP01 and LT12 should be suppressed by noqa, got: {:?}",
        violations
    );
}

// ── case insensitivity ────────────────────────────────────────────────────────

#[test]
fn noqa_rule_id_case_insensitive() {
    // Lowercase rule ID in noqa comment should still work.
    let sql = "SELECT a -- noqa: cp01\nFROM t";
    let v = check_cp01(sql);
    assert_eq!(
        v.len(),
        1,
        "expected 1 violation (FROM on line 2), got: {:?}",
        v
    );
    assert_eq!(v[0].line, 2);
}

// ── noqa combined with fmt:off ────────────────────────────────────────────────

#[test]
fn noqa_and_fmt_off_both_suppress() {
    // Line 1: noqa suppresses SELECT violation
    // Line 2-3: fmt:off block suppresses everything
    // Line 4: fmt:on, line 5: FROM is flagged again
    let sql = "SELECT a -- noqa\n-- fmt: off\nSELECT b\n-- fmt: on\nFROM t";
    let v = check_cp01(sql);
    let lines: Vec<usize> = v.iter().map(|v| v.line).collect();
    assert!(!lines.contains(&1), "line 1 suppressed by noqa");
    assert!(!lines.contains(&3), "line 3 suppressed by fmt:off");
    assert!(lines.contains(&5), "line 5 (FROM) should be flagged");
}

// ── whitespace variants ───────────────────────────────────────────────────────

#[test]
fn noqa_no_space_after_dashes() {
    // `--noqa` (no space) should also work.
    let sql = "SELECT a --noqa\nFROM t";
    let v = check_cp01(sql);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].line, 2);
}

#[test]
fn noqa_extra_space_before_colon() {
    // `-- noqa : CP01` — space before colon should not parse as targeted noqa.
    // The `:` is not immediately after `noqa`, so the trailing text is not a
    // colon-delimited rule list. This should behave as bare `-- noqa` since
    // `rest` after `noqa` is `" : CP01"` which does not strip_prefix(':').
    // In practice: parse_noqa strips leading whitespace from rest, then checks for ':'.
    // So `-- noqa : CP01` → rest after strip = `: CP01` → strip_prefix(':') succeeds.
    // This effectively treats `-- noqa : CP01` as `-- noqa: CP01`. Test confirms CP01 is suppressed.
    let sql = "SELECT a -- noqa : CP01\nFROM t";
    let v = check_cp01(sql);
    assert_eq!(
        v.len(),
        1,
        "CP01 on line 1 should be suppressed, FROM on line 2 flagged"
    );
    assert_eq!(v[0].line, 2);
}
