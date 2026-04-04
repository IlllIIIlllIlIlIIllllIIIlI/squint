use crate::node::Node;
use crate::rules::{Fix, LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// LT06 — No space between a function name and its opening parenthesis.
///
/// `sum (a)` → `sum(a)`
pub struct FunctionSpacing;

impl Rule for FunctionSpacing {
    fn id(&self) -> &'static str {
        "LT06"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();

        for i in 1..nodes.len() {
            let n = &nodes[i];
            let prev = &nodes[i - 1];

            if n.token.token_type == TokenType::BracketOpen
                && n.value == "("
                && prev.token.token_type == TokenType::Name
                && !n.prefix.is_empty()
                && !n.prefix.contains('\n')
            {
                let (line, col) = line_index.offset_to_line_col(n.token.spos);
                violations.push(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!("No space allowed between function '{}' and '('", prev.value),
                    severity: Severity::Error,
                });
            }
        }

        violations
    }

    fn fixes(&self, nodes: &[Node<'_>], _source: &str) -> Vec<Fix> {
        let mut fixes = Vec::new();
        for i in 1..nodes.len() {
            let n = &nodes[i];
            let prev = &nodes[i - 1];
            if n.token.token_type == TokenType::BracketOpen
                && n.value == "("
                && prev.token.token_type == TokenType::Name
                && !n.prefix.is_empty()
                && !n.prefix.contains('\n')
            {
                fixes.push(Fix {
                    start: n.token.spos - n.prefix.len(),
                    end: n.token.spos,
                    replacement: String::new(),
                });
            }
        }
        fixes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(FunctionSpacing, sql)
    }

    #[test]
    fn test_ok() {
        assert!(violations("select count(id) from t").is_empty());
    }

    #[test]
    fn test_space_violation() {
        let v = violations("select count (id) from t");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("count"));
    }
}
