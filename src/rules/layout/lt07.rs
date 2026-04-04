use crate::analysis::cte_definitions;
use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// LT07 — The closing bracket of a CTE definition must be on its own line.
///
/// ```sql
/// -- bad
/// WITH cte AS (SELECT 1)
/// -- good
/// WITH cte AS (
///     SELECT 1
/// )
/// ```
pub struct CteBracket;

impl Rule for CteBracket {
    fn id(&self) -> &'static str {
        "LT07"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();
        let ctes = cte_definitions(nodes);

        for cte in &ctes {
            let close_idx = cte.close_idx;

            // The closing `)` must be on its own line.
            // Since newlines are separate tokens, check that only Newline/Comment tokens
            // appear between the previous content token and the close bracket.
            let preceded_by_newline = close_idx > 0 && {
                let mut j = close_idx - 1;
                loop {
                    match nodes[j].token.token_type {
                        TokenType::Newline | TokenType::Comment => {
                            if nodes[j].token.token_type == TokenType::Newline {
                                break true;
                            }
                        }
                        _ => break false,
                    }
                    if j == 0 {
                        break false;
                    }
                    j -= 1;
                }
            } || nodes[close_idx].prefix.contains('\n');
            if !preceded_by_newline {
                let (line, col) = line_index.offset_to_line_col(nodes[close_idx].token.spos);
                violations.push(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!(
                        "Closing bracket of CTE '{}' must be on its own line",
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
        crate::rules::check_rule(CteBracket, sql)
    }

    #[test]
    fn test_ok() {
        let sql = "with cte as (\n    select 1\n)\nselect * from cte";
        assert!(violations(sql).is_empty());
    }

    #[test]
    fn test_violation() {
        let v = violations("with cte as (select 1)\nselect * from cte");
        assert_eq!(v.len(), 1);
    }
}
