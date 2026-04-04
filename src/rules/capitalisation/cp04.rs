use crate::node::Node;
use crate::rules::{Fix, LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

const LITERALS: &[&str] = &["true", "false", "null"];

/// CP04 — Boolean and null literals must be lowercase.
pub struct LiteralCasing;

impl Rule for LiteralCasing {
    fn id(&self) -> &'static str {
        "CP04"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        nodes
            .iter()
            .filter(|n| {
                // null is classified as WordOperator; true/false as Name
                matches!(
                    n.token.token_type,
                    TokenType::Name | TokenType::WordOperator
                ) && LITERALS.iter().any(|l| n.value.eq_ignore_ascii_case(l))
                    && n.value.bytes().any(|b| b.is_ascii_uppercase())
            })
            .map(|n| {
                let (line, col) = line_index.offset_to_line_col(n.token.spos);
                Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!(
                        "Literals must be lowercase: found '{}', expected '{}'",
                        n.value,
                        n.value.to_lowercase()
                    ),
                    severity: Severity::Error,
                }
            })
            .collect()
    }

    fn fixes(&self, nodes: &[Node<'_>], _source: &str) -> Vec<Fix> {
        nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.token.token_type,
                    TokenType::Name | TokenType::WordOperator
                ) && LITERALS.iter().any(|l| n.value.eq_ignore_ascii_case(l))
                    && n.value.bytes().any(|b| b.is_ascii_uppercase())
            })
            .map(|n| Fix {
                start: n.token.spos,
                end: n.token.epos,
                replacement: n.value.to_lowercase(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(LiteralCasing, sql)
    }

    #[test]
    fn test_clean() {
        assert!(violations("select true, false, null from t").is_empty());
    }

    #[test]
    fn test_uppercase_null() {
        let v = violations("select NULL from t");
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_uppercase_booleans() {
        let v = violations("select TRUE, FALSE from t");
        assert_eq!(v.len(), 2);
    }
}
