/// Tests for the `apply_fixes` utility and multi-rule integration scenarios.
use squint::linter::{fix_source, lint_source};
use squint::rules::{
    apply_fixes,
    capitalisation::{FunctionCasing, IdentifierCasing, KeywordCasing, LiteralCasing, TypeCasing},
    convention::IsNull,
    layout::{EndOfFile, FunctionSpacing, Spacing},
    Fix, Rule,
};

// ── apply_fixes ───────────────────────────────────────────────────────────────

#[test]
fn apply_fixes_empty() {
    assert_eq!(apply_fixes("select 1", vec![]), "select 1");
}

#[test]
fn apply_fixes_single() {
    let fixes = vec![Fix {
        start: 0,
        end: 6,
        replacement: "SELECT".to_string(),
    }];
    assert_eq!(apply_fixes("select 1", fixes), "SELECT 1");
}

#[test]
fn apply_fixes_multiple_non_overlapping() {
    let fixes = vec![
        Fix {
            start: 0,
            end: 6,
            replacement: "select".to_string(),
        },
        Fix {
            start: 9,
            end: 13,
            replacement: "from".to_string(),
        },
    ];
    assert_eq!(apply_fixes("SELECT a FROM t", fixes), "select a from t");
}

#[test]
fn apply_fixes_overlapping_skips_second() {
    let fixes = vec![
        Fix {
            start: 0,
            end: 6,
            replacement: "SELECT".to_string(),
        },
        Fix {
            start: 0,
            end: 6,
            replacement: "select".to_string(),
        },
    ];
    // Both fixes have the same range; whichever is applied first wins.
    let result = apply_fixes("select 1", fixes);
    assert_eq!(result.len(), "select 1".len());
}

#[test]
fn apply_fixes_adjacent_both_applied() {
    let fixes = vec![
        Fix {
            start: 0,
            end: 6,
            replacement: "SELECT".to_string(),
        },
        Fix {
            start: 7,
            end: 8,
            replacement: "b".to_string(),
        },
    ];
    assert_eq!(apply_fixes("select a from t", fixes), "SELECT b from t");
}

#[test]
fn apply_fixes_append_at_end() {
    let n = "select 1".len();
    let fixes = vec![Fix {
        start: n,
        end: n,
        replacement: "\n".to_string(),
    }];
    assert_eq!(apply_fixes("select 1", fixes), "select 1\n");
}

// ── Integration ───────────────────────────────────────────────────────────────

#[test]
fn violations_sorted_by_line_col() {
    let sql = "SELECT Id FROM MyTable";
    let rules: Vec<Box<dyn Rule>> = vec![Box::new(KeywordCasing), Box::new(IdentifierCasing)];
    let refs: Vec<&dyn Rule> = rules.iter().map(|r| r.as_ref()).collect();
    let v = lint_source(sql, &refs);
    for w in v.windows(2) {
        assert!(
            (w[0].line, w[0].col) <= (w[1].line, w[1].col),
            "violations out of order: {:?} > {:?}",
            w[0],
            w[1]
        );
    }
}

#[test]
fn multiple_cp_rules_fix_in_one_pass() {
    let sql = "SELECT Id, COUNT (Id) FROM MyTable WHERE x = NULL";
    let rules: Vec<Box<dyn Rule>> = vec![
        Box::new(KeywordCasing),
        Box::new(IdentifierCasing),
        Box::new(FunctionCasing),
        Box::new(LiteralCasing),
        Box::new(Spacing),
    ];
    let refs: Vec<&dyn Rule> = rules.iter().map(|r| r.as_ref()).collect();
    assert_eq!(
        fix_source(sql, &refs),
        "select id, count(id) from mytable where x = null"
    );
}

#[test]
fn cv05_and_cp04_fixed_together() {
    // CV05 replaces `=` with `is`; CP04 lowercases `NULL` — non-overlapping, both applied.
    let sql = "select * from t where a = NULL";
    let rules: Vec<Box<dyn Rule>> = vec![Box::new(IsNull), Box::new(LiteralCasing)];
    let refs: Vec<&dyn Rule> = rules.iter().map(|r| r.as_ref()).collect();
    assert_eq!(fix_source(sql, &refs), "select * from t where a is null");
}

#[test]
fn fix_converges_in_one_pass() {
    // After fixing, a second pass must produce no further changes.
    let sql = "SELECT  Id , COUNT (Id) FROM MyTable WHERE x = NULL";
    let rules: Vec<Box<dyn Rule>> = vec![
        Box::new(KeywordCasing),
        Box::new(IdentifierCasing),
        Box::new(FunctionCasing),
        Box::new(LiteralCasing),
        Box::new(Spacing),
        Box::new(FunctionSpacing),
        Box::new(IsNull),
        Box::new(EndOfFile),
    ];
    let refs: Vec<&dyn Rule> = rules.iter().map(|r| r.as_ref()).collect();
    let once = fix_source(sql, &refs);
    let twice = fix_source(&once, &refs);
    assert_eq!(once, twice);
}

#[test]
fn no_violations_after_fix() {
    // For every fixable rule, applying fixes then linting must produce zero violations.
    let cases: &[(&str, fn() -> Box<dyn Rule>)] = &[
        ("SELECT a FROM t", || Box::new(KeywordCasing)),
        ("select Id from t", || Box::new(IdentifierCasing)),
        ("select COUNT(id) from t", || Box::new(FunctionCasing)),
        ("select TRUE from t", || Box::new(LiteralCasing)),
        ("cast(x as VARCHAR)", || Box::new(TypeCasing)),
        ("select count (id) from t", || Box::new(FunctionSpacing)),
        ("select a , b from t", || Box::new(Spacing)),
        ("select * from t where a = null", || Box::new(IsNull)),
        ("select * from t where a != null", || Box::new(IsNull)),
        ("select 1", || Box::new(EndOfFile)),
        ("select 1\n\n", || Box::new(EndOfFile)),
    ];

    for (sql, make_rule) in cases {
        let rule = make_rule();
        let fixed = fix_source(sql, &[rule.as_ref()]);
        let rule2 = make_rule();
        let violations = lint_source(&fixed, &[rule2.as_ref()]);
        assert!(
            violations.is_empty(),
            "rule {}: after fixing {:?}, got violations: {:?}",
            make_rule().id(),
            sql,
            violations
        );
    }
}
