use crate::analysis::Clause;
use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// AM05 — Implicit comma joins in FROM clauses are forbidden.
///
/// `FROM a, b` is implicit join syntax; use explicit `JOIN` instead.
/// This rule flags comma tokens that appear directly in a top-level FROM
/// clause (bracket_depth == 0), where a comma separates table references
/// rather than appearing inside a function call or subquery.
///
/// Note: implicit joins inside subqueries or CTEs (bracket_depth > 0) are
/// not detected — this is a known limitation consistent with other rules in
/// this codebase.
pub struct ImplicitJoins;

impl Rule for ImplicitJoins {
    fn id(&self) -> &'static str {
        "AM05"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let clauses = ctx.clauses;
        let mut violations = Vec::new();

        for i in 0..nodes.len() {
            let n = &nodes[i];
            if n.token.token_type != TokenType::Comma {
                continue;
            }
            if clauses[i] != Clause::From {
                continue;
            }
            if n.bracket_depth != 0 {
                // Inside a function call or subquery — not an implicit join comma.
                continue;
            }

            let (line, col) = line_index.offset_to_line_col(n.token.spos);
            violations.push(Violation {
                line,
                col,
                rule_id: self.id(),
                message: "Implicit comma join found; use explicit JOIN syntax instead".to_string(),
                severity: Severity::Error,
            });
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(ImplicitJoins, sql)
    }

    #[test]
    fn test_explicit_join_ok() {
        assert!(
            violations("select a.id from foo as a inner join bar as b on a.id = b.id").is_empty()
        );
    }

    #[test]
    fn test_implicit_join_flagged() {
        let v = violations("select a, b from foo, bar");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule_id, "AM05");
    }

    #[test]
    fn test_three_table_implicit_join_flagged() {
        // Two commas → two violations.
        let v = violations("select * from a, b, c");
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn test_cross_join_ok() {
        assert!(violations("select a, b from foo cross join bar").is_empty());
    }

    #[test]
    fn test_select_comma_not_flagged() {
        // Commas in SELECT list must not be flagged.
        assert!(violations("select a, b, c from t").is_empty());
    }

    #[test]
    fn test_function_arg_comma_not_flagged() {
        // Commas inside a table-valued function call are at bracket_depth > 0.
        assert!(violations("select * from generate_series(1, 10) as t(n)").is_empty());
    }

    #[test]
    fn test_where_comma_not_flagged() {
        // Commas in IN(...) lists or function calls in WHERE are not FROM commas.
        assert!(violations("select a from t where a in (1, 2, 3)").is_empty());
    }
}
