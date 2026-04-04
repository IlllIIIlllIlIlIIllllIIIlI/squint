use crate::analysis::next_non_ws;
use crate::node::Node;
use crate::rules::{Fix, LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// CV05 — Use `IS NULL` / `IS NOT NULL` instead of `= NULL` / `!= NULL`.
pub struct IsNull;

impl Rule for IsNull {
    fn id(&self) -> &'static str {
        "CV05"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();

        for i in 0..nodes.len() {
            let n = &nodes[i];
            // Look for `=` or `!=`/`<>` operator
            if n.token.token_type != TokenType::Operator {
                continue;
            }
            if !matches!(n.value, "=" | "!=" | "<>") {
                continue;
            }

            if let Some(next_idx) = next_non_ws(nodes, i) {
                let next_node = &nodes[next_idx];
                // null can be Name or WordOperator depending on context
                if matches!(
                    next_node.token.token_type,
                    TokenType::Name | TokenType::WordOperator
                ) && next_node.value.eq_ignore_ascii_case("null")
                {
                    let (line, col) = line_index.offset_to_line_col(n.token.spos);
                    let preferred = if n.value == "=" {
                        "IS NULL"
                    } else {
                        "IS NOT NULL"
                    };
                    violations.push(Violation {
                        line,
                        col,
                        rule_id: self.id(),
                        message: format!("Use '{}' instead of '{} NULL'", preferred, n.value),
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
            if n.token.token_type != TokenType::Operator {
                continue;
            }
            if !matches!(n.value, "=" | "!=" | "<>") {
                continue;
            }
            if let Some(next_idx) = next_non_ws(nodes, i) {
                let next_node = &nodes[next_idx];
                if matches!(
                    next_node.token.token_type,
                    TokenType::Name | TokenType::WordOperator
                ) && next_node.value.eq_ignore_ascii_case("null")
                {
                    let replacement = if n.value == "=" {
                        "is".to_string()
                    } else {
                        "is not".to_string()
                    };
                    fixes.push(Fix {
                        start: n.token.spos,
                        end: n.token.epos,
                        replacement,
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
        crate::rules::check_rule(IsNull, sql)
    }

    #[test]
    fn test_is_null_ok() {
        assert!(violations("select * from t where a is null").is_empty());
    }

    #[test]
    fn test_eq_null_violation() {
        let v = violations("select * from t where a = null");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("IS NULL"));
    }

    #[test]
    fn test_neq_null_violation() {
        let v = violations("select * from t where a != null");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("IS NOT NULL"));
    }
}
