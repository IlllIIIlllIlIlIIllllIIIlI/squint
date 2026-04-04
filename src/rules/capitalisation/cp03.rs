use crate::node::Node;
use crate::rules::{Fix, LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// CP03 — Function names must be lowercase.
///
/// A function call is a Name token immediately followed by `(`.
pub struct FunctionCasing;

impl Rule for FunctionCasing {
    fn id(&self) -> &'static str {
        "CP03"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();
        for i in 0..nodes.len() {
            let n = &nodes[i];
            if n.token.token_type != TokenType::Name {
                continue;
            }
            // Check if next meaningful token is `(`
            let next = nodes[i + 1..].iter().find(|x| {
                x.token.token_type != TokenType::Newline && x.token.token_type != TokenType::Comment
            });
            if let Some(next_node) = next {
                if next_node.token.token_type == TokenType::BracketOpen
                    && next_node.value == "("
                    && n.value.bytes().any(|b| b.is_ascii_uppercase())
                {
                    let lower = n.value.to_lowercase();
                    let (line, col) = line_index.offset_to_line_col(n.token.spos);
                    violations.push(Violation {
                        line,
                        col,
                        rule_id: self.id(),
                        message: format!(
                            "Function names must be lowercase: found '{}', expected '{}'",
                            n.value, lower
                        ),
                        severity: Severity::Error,
                    });
                }
            }
        }
        violations
    }

    fn fixes(&self, nodes: &[Node<'_>], _source: &str) -> Vec<Fix> {
        let mut fixes = Vec::new();
        for i in 0..nodes.len() {
            let n = &nodes[i];
            if n.token.token_type != TokenType::Name {
                continue;
            }
            if !n.value.bytes().any(|b| b.is_ascii_uppercase()) {
                continue;
            }
            let next = nodes[i + 1..].iter().find(|x| {
                x.token.token_type != TokenType::Newline && x.token.token_type != TokenType::Comment
            });
            if let Some(next_node) = next {
                if next_node.token.token_type == TokenType::BracketOpen && next_node.value == "(" {
                    fixes.push(Fix {
                        start: n.token.spos,
                        end: n.token.epos,
                        replacement: n.value.to_lowercase(),
                    });
                }
            }
        }
        fixes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(FunctionCasing, sql)
    }

    #[test]
    fn test_clean() {
        assert!(violations("select count(id) from t").is_empty());
    }

    #[test]
    fn test_uppercase_function() {
        let v = violations("select COUNT(id) from t");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("COUNT"));
    }

    #[test]
    fn test_not_a_function_call() {
        // Bare name not followed by ( should not be flagged by this rule
        assert!(violations("select Id from T").is_empty());
    }
}
