/// Integration tests for `-- fmt: off` / `-- fmt: on` suppression.
///
/// Uses CP01 (KeywordCasing) as the probe rule — it has both check() and fixes(),
/// making it easy to verify both violation suppression and fix suppression.
mod common;

use squint::rules::capitalisation::KeywordCasing;

fn check(sql: &str) -> Vec<squint::rules::Violation> {
    common::check(KeywordCasing, sql)
}

fn fix(sql: &str) -> String {
    common::fix(KeywordCasing, sql)
}

// ── Inline (single-line) suppression ─────────────────────────────────────────

#[test]
fn inline_suppresses_that_line_only() {
    // Line 1 has uppercase SELECT with fmt:off — no violation.
    // Line 2 has uppercase FROM — still flagged.
    let sql = "SELECT a -- fmt: off\nFROM t";
    let v = check(sql);
    assert_eq!(
        v.len(),
        1,
        "expected 1 violation (FROM on line 2), got: {:?}",
        v
    );
    assert_eq!(v[0].line, 2);
}

#[test]
fn inline_suppresses_fix_on_that_line() {
    let sql = "SELECT a -- fmt: off\nFROM t";
    let fixed = fix(sql);
    // SELECT on line 1 must NOT be lowercased; FROM on line 2 must be lowercased.
    assert!(fixed.contains("SELECT"), "SELECT should be preserved");
    assert!(fixed.contains("from"), "FROM should be lowercased");
}

#[test]
fn inline_does_not_affect_preceding_line() {
    // Line 1 is clean. Line 2 has violation + inline fmt:off. Line 3 has a violation.
    let sql = "select a\nSELECT b -- fmt: off\nFROM t";
    let v = check(sql);
    // Only line 3 (FROM) should be flagged.
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].line, 3);
}

// ── Block suppression ────────────────────────────────────────────────────────

#[test]
fn block_suppresses_violations_inside() {
    let sql = "select a\n-- fmt: off\nSELECT b\nFROM t\n-- fmt: on\nFROM u";
    let v = check(sql);
    // Only FROM on line 6 should be flagged.
    assert_eq!(
        v.len(),
        1,
        "expected 1 violation (FROM on line 6), got: {:?}",
        v
    );
    assert_eq!(v[0].line, 6);
}

#[test]
fn block_suppresses_fixes_inside() {
    let sql = "select a\n-- fmt: off\nSELECT b\n-- fmt: on\nFROM t";
    let fixed = fix(sql);
    assert!(
        fixed.contains("SELECT b"),
        "SELECT inside block must be preserved"
    );
    assert!(
        fixed.contains("from t"),
        "FROM after block must be lowercased"
    );
}

#[test]
fn fmt_off_line_itself_is_suppressed() {
    // The `-- fmt: off` line is line 1. Any violation on line 1 is suppressed.
    // (No keyword on the fmt:off line here, just verifying line 2 inside block is suppressed.)
    let sql = "-- fmt: off\nSELECT a\n-- fmt: on\nFROM t";
    let v = check(sql);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].line, 4); // only FROM on line 4
}

#[test]
fn fmt_on_line_itself_is_suppressed_violation_after_is_not() {
    // The `-- fmt: on` line closes the block. Any violation on the fmt:on line is
    // inside the region (inclusive). The very next line after is linted normally.
    let sql = "-- fmt: off\nSELECT a\n-- fmt: on\nFROM t";
    let v = check(sql);
    // Line 4 (FROM) is after the block and must be flagged.
    assert!(v.iter().any(|v| v.line == 4));
    // Lines 1-3 are inside the block (inclusive of fmt:on line).
    assert!(!v.iter().any(|v| v.line <= 3));
}

// ── To-EOF suppression ───────────────────────────────────────────────────────

#[test]
fn standalone_no_fmt_on_suppresses_to_eof() {
    let sql = "select a\n-- fmt: off\nSELECT b\nFROM t";
    let v = check(sql);
    // Line 1 is before fmt:off — no violation (already lowercase).
    // Lines 2-4 are inside the suppressed region.
    assert!(v.is_empty(), "expected no violations, got: {:?}", v);
}

#[test]
fn standalone_no_fmt_on_suppresses_fixes_to_eof() {
    let sql = "select a\n-- fmt: off\nSELECT b\nFROM t";
    let fixed = fix(sql);
    assert!(
        fixed.contains("SELECT b"),
        "SELECT after fmt:off must be preserved"
    );
    assert!(
        fixed.contains("FROM t"),
        "FROM after fmt:off must be preserved"
    );
}

#[test]
fn full_file_fmt_off_suppresses_everything() {
    let sql = "-- fmt: off\nSELECT a\nFROM t\nWHERE x = 1";
    let v = check(sql);
    assert!(
        v.is_empty(),
        "expected no violations in fully-suppressed file, got: {:?}",
        v
    );
}

#[test]
fn full_file_fmt_off_suppresses_all_fixes() {
    let sql = "-- fmt: off\nSELECT a\nFROM t";
    let fixed = fix(sql);
    assert_eq!(
        fixed, sql,
        "nothing should be changed in a fully-suppressed file"
    );
}

// ── Multiple pairs ────────────────────────────────────────────────────────────

#[test]
fn multiple_pairs_each_suppressed_independently() {
    let sql = "FROM a\n-- fmt: off\nSELECT b\n-- fmt: on\nFROM c\n-- fmt: off\nSELECT d\n-- fmt: on\nFROM e";
    let v = check(sql);
    // Lines 1, 5, 9 (FROM a/c/e) are outside fmt:off blocks and must be flagged.
    // Lines 3, 7 (SELECT b/d) are inside blocks and must not be flagged.
    let flagged_lines: Vec<usize> = v.iter().map(|v| v.line).collect();
    assert!(
        flagged_lines.contains(&1),
        "line 1 (FROM) should be flagged"
    );
    assert!(
        flagged_lines.contains(&5),
        "line 5 (FROM) should be flagged"
    );
    assert!(
        flagged_lines.contains(&9),
        "line 9 (FROM) should be flagged"
    );
    assert!(
        !flagged_lines.contains(&3),
        "line 3 (SELECT inside block) must not be flagged"
    );
    assert!(
        !flagged_lines.contains(&7),
        "line 7 (SELECT inside block) must not be flagged"
    );
}
