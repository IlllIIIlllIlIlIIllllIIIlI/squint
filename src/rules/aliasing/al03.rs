use crate::analysis::select_items_with_clauses;
use crate::node::Node;
use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// AL03 — Column expressions in SELECT must have an alias.
///
/// Simple column references (bare Name or QuotedName) are exempt when
/// `allow_scalar` would apply; function calls and expressions must be aliased.
pub struct ExpressionAliases;

impl Rule for ExpressionAliases {
    fn id(&self) -> &'static str {
        "AL03"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        select_items_with_clauses(nodes, ctx.clauses)
            .into_iter()
            .filter(|item| item.alias.is_none())
            .filter(|item| is_expression(nodes, item.start, item.end))
            .map(|item| {
                let (line, col) = line_index.offset_to_line_col(nodes[item.start].token.spos);
                Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: "Column expression must have an alias".to_string(),
                    severity: Severity::Error,
                }
            })
            .collect()
    }
}

/// Returns true if the span `start..=end` is a non-trivial expression
/// (i.e., not a bare column reference or `*`).
fn is_expression(nodes: &[Node<'_>], start: usize, end: usize) -> bool {
    if start >= end {
        return false;
    }
    // Check for any token that makes it an expression
    if nodes[start..=end].iter().any(|n| {
        matches!(
            n.token.token_type,
            TokenType::BracketOpen | TokenType::Operator | TokenType::Star | TokenType::Dot
        )
    }) {
        return true;
    }
    // Multi-token span that is not `schema.table.col` → expression
    let meaningful: Vec<_> = (start..=end)
        .filter(|&i| {
            !matches!(
                nodes[i].token.token_type,
                TokenType::Newline | TokenType::Comment | TokenType::Dot
            )
        })
        .collect();
    meaningful.len() > 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(ExpressionAliases, sql)
    }

    #[test]
    fn test_simple_col_ok() {
        assert!(violations("select a, b from t").is_empty());
    }

    #[test]
    fn test_aliased_expr_ok() {
        assert!(violations("select count(id) as cnt from t").is_empty());
    }

    #[test]
    fn test_unaliased_function() {
        let v = violations("select count(id) from t");
        assert_eq!(v.len(), 1);
    }
}
