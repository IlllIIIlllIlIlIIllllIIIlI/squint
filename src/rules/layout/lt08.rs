use crate::analysis::cte_definitions;
use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// LT08 — A blank line is required after the closing bracket of a CTE definition.
///
/// The blank line must appear before the comma (if another CTE follows) or
/// before the main query SELECT.
pub struct CteNewline;

impl Rule for CteNewline {
    fn id(&self) -> &'static str {
        "LT08"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();
        let ctes = cte_definitions(nodes);

        for cte in &ctes {
            let close_idx = cte.close_idx;

            // Check that between `)` and the next comma/SELECT there are ≥2 newlines
            // (i.e., at least one blank line).
            let mut newline_count = 0usize;
            for node in &nodes[close_idx + 1..] {
                match node.token.token_type {
                    TokenType::Newline => newline_count += 1,
                    TokenType::Comment => {}
                    _ => break,
                }
            }

            if newline_count < 2 {
                let (line, col) = line_index.offset_to_line_col(nodes[close_idx].token.spos);
                violations.push(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!(
                        "Expected a blank line after closing bracket of CTE '{}'",
                        cte.name
                    ),
                    severity: Severity::Error,
                });
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(CteNewline, sql)
    }

    #[test]
    fn test_ok() {
        let sql = "with cte as (\n    select 1\n)\n\nselect * from cte";
        assert!(violations(sql).is_empty());
    }

    #[test]
    fn test_no_blank_line() {
        let v = violations("with cte as (\n    select 1\n)\nselect * from cte");
        assert_eq!(v.len(), 1);
    }
}
