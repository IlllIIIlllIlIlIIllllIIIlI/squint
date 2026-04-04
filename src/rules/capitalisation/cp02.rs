use crate::node::Node;
use crate::rules::{Fix, LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// CP02 — Unquoted identifiers must be lowercase.
pub struct IdentifierCasing;

impl Rule for IdentifierCasing {
    fn id(&self) -> &'static str {
        "CP02"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        nodes
            .iter()
            .filter(|n| n.token.token_type == TokenType::Name)
            .filter(|n| n.value.bytes().any(|b| b.is_ascii_uppercase()))
            .map(|n| {
                let (line, col) = line_index.offset_to_line_col(n.token.spos);
                Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!(
                        "Identifiers must be lowercase: found '{}', expected '{}'",
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
            .filter(|n| n.token.token_type == TokenType::Name)
            .filter(|n| n.value.bytes().any(|b| b.is_ascii_uppercase()))
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
        crate::rules::check_rule(IdentifierCasing, sql)
    }

    #[test]
    fn test_clean() {
        assert!(violations("select id from t").is_empty());
    }

    #[test]
    fn test_uppercase_identifier() {
        let v = violations("select Id from T");
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn test_quoted_exempt() {
        assert!(violations(r#"select "MyCol" from "MyTable""#).is_empty());
    }
}
