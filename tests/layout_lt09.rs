/// Integration tests for LT09 (clause ordering).
mod common;
use common::check;
use squint::rules::layout::ClauseOrdering;

// ── LT09 — clause ordering ────────────────────────────────────────────────────

#[test]
fn lt09_full_standard_order_ok() {
    assert!(check(
        ClauseOrdering,
        "select a, count(*)\nfrom t\nwhere a > 0\ngroup by a\nhaving count(*) > 1\norder by a\nlimit 10\n"
    )
    .is_empty());
}

#[test]
fn lt09_select_only_ok() {
    assert!(check(ClauseOrdering, "select 1\n").is_empty());
}

#[test]
fn lt09_select_from_ok() {
    assert!(check(ClauseOrdering, "select a from t\n").is_empty());
}

#[test]
fn lt09_select_from_where_ok() {
    assert!(check(ClauseOrdering, "select a from t where a = 1\n").is_empty());
}

#[test]
fn lt09_skipped_clauses_ok() {
    // GROUP BY without WHERE is fine
    assert!(check(
        ClauseOrdering,
        "select a, count(*) from t group by a\n"
    )
    .is_empty());
}

#[test]
fn lt09_where_before_from_flagged() {
    let v = check(ClauseOrdering, "select a where a = 1 from t\n");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].rule_id, "LT09");
    assert!(v[0].message.contains("FROM"));
}

#[test]
fn lt09_order_before_where_flagged() {
    let v = check(ClauseOrdering, "select a from t order by a where a = 1\n");
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("WHERE"));
}

#[test]
fn lt09_group_after_order_flagged() {
    let v = check(
        ClauseOrdering,
        "select a from t order by a group by a\n",
    );
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("GROUP"));
}

#[test]
fn lt09_having_before_group_flagged() {
    let v = check(
        ClauseOrdering,
        "select a from t where a > 0 having count(*) > 1 group by a\n",
    );
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("GROUP"));
}

#[test]
fn lt09_limit_before_order_flagged() {
    let v = check(
        ClauseOrdering,
        "select a from t limit 10 order by a\n",
    );
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("ORDER"));
}

#[test]
fn lt09_join_after_where_flagged() {
    let v = check(
        ClauseOrdering,
        "select a from t where a = 1 join u on t.id = u.id\n",
    );
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("JOIN"));
}

#[test]
fn lt09_union_resets_state() {
    // Second SELECT starts a fresh ordering context
    assert!(check(
        ClauseOrdering,
        "select a from t order by a\nunion all\nselect b from u where b = 1\n"
    )
    .is_empty());
}

#[test]
fn lt09_cte_outer_query_ok() {
    assert!(check(
        ClauseOrdering,
        "with cte as (select a from t)\nselect b from cte where b = 1 order by b\n"
    )
    .is_empty());
}

#[test]
fn lt09_subquery_outer_order_ok() {
    // Subqueries are at bracket_depth > 0 and exempt; outer query is checked
    assert!(check(
        ClauseOrdering,
        "select a from (select b from u) sub where a = 1\n"
    )
    .is_empty());
}

#[test]
fn lt09_violation_position_reported() {
    // "select a where a = 1 " is 21 chars; FROM starts at byte 21
    let v = check(ClauseOrdering, "select a where a = 1 from t\n");
    assert_eq!(v[0].line, 1);
    assert_eq!(v[0].col, 22);
}
