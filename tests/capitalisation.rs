mod common;
use common::{check, fix};
use squint::rules::capitalisation::{
    FunctionCasing, IdentifierCasing, KeywordCasing, LiteralCasing, TypeCasing,
};

// ── CP01 — Keyword casing ─────────────────────────────────────────────────────

#[test]
fn cp01_clean() {
    assert!(check(KeywordCasing, "select a from t where a = 1 and b = 2").is_empty());
}

#[test]
fn cp01_uppercase_keywords() {
    let v = check(KeywordCasing, "SELECT a FROM t");
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].rule_id, "CP01");
    assert!(v[0].message.contains("SELECT"));
    assert!(v[1].message.contains("FROM"));
}

#[test]
fn cp01_boolean_operator() {
    let v = check(KeywordCasing, "select a from t where a = 1 AND b = 2");
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("AND"));
}

#[test]
fn cp01_line_col() {
    let v = check(KeywordCasing, "SELECT id");
    assert_eq!(v[0].line, 1);
    assert_eq!(v[0].col, 1);
}

#[test]
fn cp01_multiline_col() {
    let v = check(KeywordCasing, "select id\nFROM t");
    assert_eq!(v[0].line, 2);
    assert_eq!(v[0].col, 1);
}

#[test]
fn cp01_fix() {
    assert_eq!(
        fix(KeywordCasing, "SELECT a FROM t WHERE x = 1"),
        "select a from t where x = 1"
    );
}

#[test]
fn cp01_fix_idempotent() {
    let sql = "select a from t";
    assert_eq!(fix(KeywordCasing, sql), sql);
}

// ── CP02 — Identifier casing ──────────────────────────────────────────────────

#[test]
fn cp02_clean() {
    assert!(check(IdentifierCasing, "select id from t").is_empty());
}

#[test]
fn cp02_uppercase_identifier() {
    let v = check(IdentifierCasing, "select Id from MyTable");
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].rule_id, "CP02");
}

#[test]
fn cp02_quoted_exempt() {
    assert!(check(IdentifierCasing, r#"select "MyCol" from t"#).is_empty());
}

#[test]
fn cp02_fix() {
    assert_eq!(
        fix(IdentifierCasing, "select Id from MyTable"),
        "select id from mytable"
    );
}

// ── CP03 — Function casing ────────────────────────────────────────────────────

#[test]
fn cp03_clean() {
    assert!(check(FunctionCasing, "select count(id) from t").is_empty());
}

#[test]
fn cp03_uppercase_function() {
    let v = check(FunctionCasing, "select COUNT(id) from t");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "CP03");
    assert!(v[0].message.contains("COUNT"));
}

#[test]
fn cp03_bare_name_not_flagged() {
    // A Name not followed by ( is CP02's job, not CP03's
    assert!(check(FunctionCasing, "select Id from T").is_empty());
}

#[test]
fn cp03_fix() {
    assert_eq!(
        fix(FunctionCasing, "select COUNT(id) from t"),
        "select count(id) from t"
    );
}

// ── CP04 — Literal casing ─────────────────────────────────────────────────────

#[test]
fn cp04_clean() {
    assert!(check(LiteralCasing, "select true, false, null from t").is_empty());
}

#[test]
fn cp04_uppercase_literals() {
    let v = check(LiteralCasing, "select TRUE, FALSE, NULL from t");
    assert_eq!(v.len(), 3);
    assert_eq!(v[0].rule_id, "CP04");
}

#[test]
fn cp04_fix() {
    assert_eq!(
        fix(LiteralCasing, "select TRUE, NULL from t"),
        "select true, null from t"
    );
}

// ── CP05 — Type casing ────────────────────────────────────────────────────────

#[test]
fn cp05_clean() {
    assert!(check(TypeCasing, "cast(x as varchar)").is_empty());
}

#[test]
fn cp05_uppercase_type() {
    let v = check(TypeCasing, "cast(x as VARCHAR)");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "CP05");
}

#[test]
fn cp05_multiple_types() {
    let v = check(TypeCasing, "cast(x as INT) + cast(y as BIGINT)");
    assert_eq!(v.len(), 2);
}

#[test]
fn cp05_fix() {
    assert_eq!(fix(TypeCasing, "cast(x as VARCHAR)"), "cast(x as varchar)");
}
