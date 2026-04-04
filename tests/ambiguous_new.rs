/// Integration tests for AM05 (implicit comma joins).
mod common;
use common::check;
use squint::rules::ambiguous::ImplicitJoins;

// ── AM05 — implicit comma joins ───────────────────────────────────────────────

#[test]
fn am05_explicit_join_ok() {
    assert!(check(
        ImplicitJoins,
        "select a.id from foo as a inner join bar as b on a.id = b.id\n"
    )
    .is_empty());
}

#[test]
fn am05_left_join_ok() {
    assert!(check(
        ImplicitJoins,
        "select a.id from foo as a left join bar as b on a.id = b.id\n"
    )
    .is_empty());
}

#[test]
fn am05_cross_join_ok() {
    assert!(check(ImplicitJoins, "select a, b from foo cross join bar\n").is_empty());
}

#[test]
fn am05_single_table_ok() {
    assert!(check(ImplicitJoins, "select a, b, c from t\n").is_empty());
}

#[test]
fn am05_implicit_join_flagged() {
    let v = check(ImplicitJoins, "select a, b from foo, bar\n");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "AM05");
}

#[test]
fn am05_three_table_implicit_flagged() {
    let v = check(ImplicitJoins, "select * from a, b, c\n");
    assert_eq!(v.len(), 2);
}

#[test]
fn am05_select_commas_not_flagged() {
    assert!(check(ImplicitJoins, "select a, b, c from t\n").is_empty());
}

#[test]
fn am05_where_in_list_not_flagged() {
    assert!(check(ImplicitJoins, "select a from t where a in (1, 2, 3)\n").is_empty());
}

#[test]
fn am05_function_arg_not_flagged() {
    assert!(check(
        ImplicitJoins,
        "select * from generate_series(1, 10) as t(n)\n"
    )
    .is_empty());
}

#[test]
fn am05_violation_position_reported() {
    // "select a, b from foo" is 20 chars; comma is at byte 20 (0-based), col 21.
    let v = check(ImplicitJoins, "select a, b from foo, bar\n");
    assert_eq!(v[0].line, 1);
    assert_eq!(v[0].col, 21);
}

#[test]
fn am05_implicit_join_with_where_flagged() {
    let v = check(
        ImplicitJoins,
        "select a.id, b.name from foo a, bar b where a.id = b.id\n",
    );
    assert_eq!(v.len(), 1);
}

#[test]
fn am05_multiline_implicit_join_flagged() {
    let sql = "select\n    a.id,\n    b.name\nfrom foo a,\n    bar b\n";
    let v = check(ImplicitJoins, sql);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].line, 4); // comma is on line 4
}
