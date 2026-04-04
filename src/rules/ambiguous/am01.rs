use crate::analysis::Clause;
use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// AM01 — DISTINCT is redundant when GROUP BY is also present.
pub struct DistinctGroupBy;

impl Rule for DistinctGroupBy {
    fn id(&self) -> &'static str {
        "AM01"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let clauses = ctx.clauses;
        let mut violations = Vec::new();

        // Find each SELECT DISTINCT at top level
        for i in 0..nodes.len() {
            let n = &nodes[i];
            if n.token.token_type != TokenType::StatementStart
                || !n.value.eq_ignore_ascii_case("select")
                || n.bracket_depth != 0
            {
                continue;
            }

            // Look for DISTINCT in the SELECT clause tokens at the same depth
            let mut distinct_idx: Option<usize> = None;
            let mut has_group_by = false;

            for j in (i + 1)..nodes.len() {
                if nodes[j].bracket_depth != 0 {
                    continue;
                }
                // Another top-level SELECT ends this statement
                if nodes[j].token.token_type == TokenType::StatementStart
                    && nodes[j].value.eq_ignore_ascii_case("select")
                    && j != i
                {
                    break;
                }
                if clauses[j] == Clause::Select && nodes[j].value.eq_ignore_ascii_case("distinct") {
                    distinct_idx = Some(j);
                }
                if clauses[j] == Clause::GroupBy {
                    has_group_by = true;
                }
            }

            if let Some(idx) = distinct_idx {
                if has_group_by {
                    let (line, col) = line_index.offset_to_line_col(nodes[idx].token.spos);
                    violations.push(Violation {
                        line,
                        col,
                        rule_id: self.id(),
                        message: "DISTINCT is redundant when GROUP BY is present".to_string(),
                        severity: Severity::Error,
                    });
                }
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(DistinctGroupBy, sql)
    }

    #[test]
    fn test_ok() {
        assert!(violations("select distinct a from t").is_empty());
    }

    #[test]
    fn test_distinct_with_group_by() {
        let v = violations("select distinct a from t group by a");
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_group_by_without_distinct_ok() {
        assert!(violations("select a from t group by a").is_empty());
    }
}
