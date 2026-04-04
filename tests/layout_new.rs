/// Integration tests for LT03 (trailing whitespace).
mod common;
use common::{check, fix};
use squint::rules::layout::TrailingWhitespace;

// ── LT03 — trailing whitespace ────────────────────────────────────────────────

#[test]
fn lt03_clean_ok() {
    assert!(check(TrailingWhitespace, "select a\nfrom t\n").is_empty());
}

#[test]
fn lt03_trailing_spaces_flagged() {
    let v = check(TrailingWhitespace, "select a   \nfrom t\n");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "LT03");
    assert_eq!(v[0].line, 1);
}

#[test]
fn lt03_trailing_tab_flagged() {
    let v = check(TrailingWhitespace, "select a\t\nfrom t\n");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].line, 1);
}

#[test]
fn lt03_multiple_lines_all_flagged() {
    let v = check(TrailingWhitespace, "select a  \nfrom t  \nwhere x = 1  \n");
    assert_eq!(v.len(), 3);
    let lines: Vec<usize> = v.iter().map(|v| v.line).collect();
    assert_eq!(lines, vec![1, 2, 3]);
}

#[test]
fn lt03_only_dirty_lines_flagged() {
    let v = check(TrailingWhitespace, "select a\nfrom t  \nwhere x = 1\n");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].line, 2);
}

#[test]
fn lt03_col_at_first_trailing_space() {
    // "select a" is 8 chars; trailing spaces begin at col 9.
    let v = check(TrailingWhitespace, "select a   \n");
    assert_eq!(v[0].col, 9);
}

#[test]
fn lt03_fix_removes_trailing_spaces() {
    assert_eq!(
        fix(TrailingWhitespace, "select a   \nfrom t\n"),
        "select a\nfrom t\n"
    );
}

#[test]
fn lt03_fix_multiple_lines() {
    assert_eq!(
        fix(TrailingWhitespace, "select a  \nfrom t  \n"),
        "select a\nfrom t\n"
    );
}

#[test]
fn lt03_fix_mixed_tabs_and_spaces() {
    assert_eq!(
        fix(TrailingWhitespace, "select a \t \nfrom t\n"),
        "select a\nfrom t\n"
    );
}

#[test]
fn lt03_empty_file_ok() {
    assert!(check(TrailingWhitespace, "").is_empty());
}

#[test]
fn lt03_line_with_only_spaces_flagged() {
    // A line that is entirely spaces (e.g. blank line with indent artifacts).
    let v = check(TrailingWhitespace, "select a\n   \nfrom t\n");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].line, 2);
    assert_eq!(v[0].col, 1);
}

#[test]
fn lt03_fix_line_with_only_spaces() {
    assert_eq!(
        fix(TrailingWhitespace, "select a\n   \nfrom t\n"),
        "select a\n\nfrom t\n"
    );
}
