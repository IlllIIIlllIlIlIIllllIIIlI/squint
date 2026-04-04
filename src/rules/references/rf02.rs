use crate::analysis::Clause;
use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// RF02 — Wildcard (`*`) column references are not allowed in SELECT.
///
/// Both bare `SELECT *` and qualified `SELECT t.*` are flagged. The only
/// permitted use of `*` in a SELECT clause is inside a function call such
/// as `COUNT(*)`, which is excluded by checking that the preceding token is
/// not an opening parenthesis.
pub struct WildcardColumns;

impl Rule for WildcardColumns {
    fn id(&self) -> &'static str {
        "RF02"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let clauses = ctx.clauses;
        let mut violations = Vec::new();

        for i in 0..nodes.len() {
            let n = &nodes[i];
            if n.token.token_type != TokenType::Star {
                continue;
            }
            if clauses[i] != Clause::Select {
                continue;
            }

            // A wildcard star must be at the same bracket depth as the SELECT
            // keyword that opened this clause. Stars at greater depth are inside
            // function call parentheses (e.g. COUNT(*), COUNT(DISTINCT *)).
            let select_at_same_depth = nodes[..i].iter().rev().any(|p| {
                p.token.token_type == TokenType::StatementStart
                    && p.value.eq_ignore_ascii_case("select")
                    && p.bracket_depth == n.bracket_depth
            });
            if !select_at_same_depth {
                continue;
            }

            // If the preceding token is a value-producing expression token,
            // the star is an arithmetic operator (e.g. `a * b`), not a wildcard.
            let prev = nodes[..i]
                .iter()
                .rev()
                .find(|p| !p.token.token_type.is_whitespace_or_comment());
            if let Some(
                TokenType::Name
                | TokenType::QuotedName
                | TokenType::Number
                | TokenType::BracketClose
                | TokenType::Data,
            ) = prev.map(|p| p.token.token_type)
            {
                continue;
            }

            let (line, col) = line_index.offset_to_line_col(n.token.spos);
            violations.push(Violation {
                line,
                col,
                rule_id: self.id(),
                message: "Wildcard (*) column reference is not allowed; list columns explicitly"
                    .to_string(),
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
        crate::rules::check_rule(WildcardColumns, sql)
    }

    #[test]
    fn test_bare_star_flagged() {
        let v = violations("select * from t");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule_id, "RF02");
    }

    #[test]
    fn test_qualified_star_flagged() {
        let v = violations("select t.* from t");
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_count_star_ok() {
        assert!(violations("select count(*) from t").is_empty());
    }

    #[test]
    fn test_count_distinct_star_ok() {
        assert!(violations("select count(distinct *) from t").is_empty());
    }

    #[test]
    fn test_explicit_columns_ok() {
        assert!(violations("select a, b, c from t").is_empty());
    }

    #[test]
    fn test_arithmetic_multiplication_ok() {
        // `*` used as arithmetic operator in SELECT must not be flagged.
        assert!(violations("select a * b from t").is_empty());
    }

    #[test]
    fn test_arithmetic_in_expression_ok() {
        assert!(violations("select (a + b) * c from t").is_empty());
    }

    #[test]
    fn test_star_with_other_columns_flagged() {
        let v = violations("select a, * from t");
        assert_eq!(v.len(), 1);
    }
}
