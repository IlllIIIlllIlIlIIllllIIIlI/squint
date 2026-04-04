use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// LT09 — SQL clauses must appear in the standard order:
/// SELECT → FROM/JOIN → WHERE → GROUP BY → HAVING → ORDER BY → LIMIT
pub struct ClauseOrdering;

impl Rule for ClauseOrdering {
    fn id(&self) -> &'static str {
        "LT09"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let nodes = ctx.nodes;
        let mut violations = Vec::new();
        let mut current_max: u8 = 0;
        let mut in_select = false;

        for n in nodes {
            // Only check top-level clauses
            if n.bracket_depth != 0 {
                continue;
            }

            // Handle statement-start keywords
            if n.token.token_type == TokenType::StatementStart {
                if n.value.eq_ignore_ascii_case("select") {
                    in_select = true;
                    current_max = 1;
                } else if !n.value.eq_ignore_ascii_case("with") {
                    // INSERT, UPDATE, DELETE, etc. — not a SELECT context
                    in_select = false;
                    current_max = 0;
                }
                // WITH is a preamble; don't reset in_select
                continue;
            }

            if !in_select {
                continue;
            }

            if let Some(rank) = clause_rank(n.value) {
                if rank < current_max {
                    let (line, col) = ctx.line_index.offset_to_line_col(n.token.spos);
                    violations.push(Violation {
                        line,
                        col,
                        rule_id: self.id(),
                        message: format!(
                            "Clause '{}' is out of order (found after {})",
                            n.value.to_uppercase(),
                            rank_name(current_max)
                        ),
                        severity: Severity::Error,
                    });
                } else {
                    current_max = rank;
                }
            }
        }

        violations
    }
}

/// Returns the expected ordering rank for a clause-starting keyword, or None
/// if the keyword doesn't start a tracked clause.
fn clause_rank(value: &str) -> Option<u8> {
    // FROM and all JOIN variants share rank 2
    if value.eq_ignore_ascii_case("from") {
        return Some(2);
    }
    if matches_any_ci(
        value,
        &["join", "inner", "left", "right", "full", "outer", "cross"],
    ) {
        return Some(2);
    }
    if value.eq_ignore_ascii_case("where") {
        return Some(3);
    }
    if value.eq_ignore_ascii_case("group") {
        return Some(4);
    }
    if value.eq_ignore_ascii_case("having") {
        return Some(5);
    }
    if value.eq_ignore_ascii_case("order") {
        return Some(6);
    }
    if value.eq_ignore_ascii_case("limit") {
        return Some(7);
    }
    None
}

fn rank_name(rank: u8) -> &'static str {
    match rank {
        1 => "SELECT",
        2 => "FROM/JOIN",
        3 => "WHERE",
        4 => "GROUP BY",
        5 => "HAVING",
        6 => "ORDER BY",
        _ => "LIMIT",
    }
}

fn matches_any_ci(value: &str, list: &[&str]) -> bool {
    list.iter().any(|k| value.eq_ignore_ascii_case(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(ClauseOrdering, sql)
    }

    #[test]
    fn test_standard_order_ok() {
        assert!(violations(
            "select a from t where a = 1 group by a having a > 0 order by a limit 10"
        )
        .is_empty());
    }

    #[test]
    fn test_select_only_ok() {
        assert!(violations("select 1").is_empty());
    }

    #[test]
    fn test_select_from_ok() {
        assert!(violations("select a from t").is_empty());
    }

    #[test]
    fn test_where_before_from_flagged() {
        let v = violations("select a where a = 1 from t");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule_id, "LT09");
        assert!(v[0].message.contains("FROM"));
    }

    #[test]
    fn test_order_before_where_flagged() {
        let v = violations("select a from t order by a where a = 1");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("WHERE"));
    }

    #[test]
    fn test_group_after_order_flagged() {
        let v = violations("select a from t order by a group by a");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("GROUP"));
    }

    #[test]
    fn test_union_resets_state() {
        // Second SELECT resets ordering; WHERE after FROM in second query is ok
        assert!(violations(
            "select a from t\nunion all\nselect b from u where b = 1"
        )
        .is_empty());
    }

    #[test]
    fn test_join_after_where_flagged() {
        let v = violations("select a from t where a = 1 join u on t.id = u.id");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("JOIN"));
    }

    #[test]
    fn test_subquery_not_flagged() {
        // WHERE inside a subquery should not be checked against outer clause order
        assert!(violations(
            "select a from (select b where b = 1 from u) sub where a = 1"
        )
        .is_empty());
    }

    #[test]
    fn test_cte_outer_query_ok() {
        assert!(violations(
            "with cte as (select a from t)\nselect b from cte where b = 1 order by b"
        )
        .is_empty());
    }

    #[test]
    fn test_multiple_violations() {
        // ORDER before WHERE and GROUP before having — actually wait,
        // let's do: limit before order, order before group
        let v = violations("select a from t limit 10 order by a group by a");
        assert_eq!(v.len(), 2); // ORDER after LIMIT, GROUP after LIMIT/ORDER
    }
}
