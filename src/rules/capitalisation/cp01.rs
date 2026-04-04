use crate::node::Node;
use crate::rules::{Fix, LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

#[inline(always)]
fn is_keyword_type(tt: TokenType) -> bool {
    matches!(
        tt,
        TokenType::StatementStart
            | TokenType::UntermintedKeyword
            | TokenType::WordOperator
            | TokenType::BooleanOperator
            | TokenType::SetOperator
            | TokenType::On
    )
}

/// CP01 — SQL keywords must be lowercase.
pub struct KeywordCasing;

impl Rule for KeywordCasing {
    fn id(&self) -> &'static str {
        "CP01"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        nodes
            .iter()
            .filter(|n| is_keyword_type(n.token.token_type))
            .filter(|n| n.value.bytes().any(|b| b.is_ascii_uppercase()))
            .map(|n| {
                let (line, col) = line_index.offset_to_line_col(n.token.spos);
                Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!(
                        "Keywords must be lowercase: found '{}', expected '{}'",
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
            .filter(|n| is_keyword_type(n.token.token_type))
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
        crate::rules::check_rule(KeywordCasing, sql)
    }

    #[test]
    fn test_no_violations_on_lowercase() {
        assert!(violations("select id from t where x = 1").is_empty());
    }

    #[test]
    fn test_violations_on_uppercase_keywords() {
        let v = violations("SELECT id FROM t");
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].rule_id, "CP01");
        assert!(v[0].message.contains("SELECT"));
        assert!(v[1].message.contains("FROM"));
    }

    #[test]
    fn test_boolean_operator_casing() {
        let v = violations("select id from t where a = 1 AND b = 2");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("AND"));
    }

    #[test]
    fn test_line_col_reported() {
        let v = violations("SELECT id");
        assert_eq!(v[0].line, 1);
        assert_eq!(v[0].col, 1);
    }

    #[test]
    fn test_multiline_col() {
        let v = violations("select id\nFROM t");
        assert_eq!(v[0].line, 2);
        assert_eq!(v[0].col, 1);
    }

    #[test]
    fn test_identifier_not_flagged_by_cp01() {
        // Identifiers are CP02's job now
        assert!(violations("select Id from T").is_empty());
    }
}
