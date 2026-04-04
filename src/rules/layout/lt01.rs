use crate::node::Node;
use crate::rules::{Fix, LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// LT01 — Improper spacing between tokens.
///
/// Checks:
/// - No space before a comma
/// - Exactly one space after a comma (within the same line)
/// - No multiple consecutive spaces between tokens on the same line
/// - No space between a function name and its opening `(`
pub struct Spacing;

impl Rule for Spacing {
    fn id(&self) -> &'static str {
        "LT01"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();

        for i in 0..nodes.len() {
            let n = &nodes[i];

            // Skip Jinja/comment/newline tokens — their whitespace is intentional
            if n.token.token_type.is_whitespace_or_comment() || n.token.token_type.is_jinja() {
                continue;
            }

            // No space before comma
            if n.token.token_type == TokenType::Comma {
                let prefix = &n.prefix;
                if !prefix.is_empty() && !prefix.contains('\n') {
                    let (line, col) = line_index.offset_to_line_col(n.token.spos - prefix.len());
                    violations.push(Violation {
                        line,
                        col,
                        rule_id: self.id(),
                        message: "Unexpected whitespace before ','".to_string(),
                        severity: Severity::Error,
                    });
                }
            }

            // No multiple consecutive spaces mid-line.
            // Skip if previous token is a Newline (this is indentation, handled by LT02).
            let prefix = &n.prefix;
            let prev_is_newline = i > 0 && nodes[i - 1].token.token_type == TokenType::Newline;
            if !prefix.contains('\n') && prefix.len() > 1 && !prev_is_newline {
                let (line, col) = line_index.offset_to_line_col(n.token.spos - prefix.len());
                violations.push(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!(
                        "Expected single space before '{}', found {} spaces",
                        n.value,
                        prefix.len()
                    ),
                    severity: Severity::Error,
                });
            }

            // No space between function name and `(`
            if n.token.token_type == TokenType::BracketOpen && n.value == "(" {
                // Check if previous token is a Name (function call)
                if let Some(prev) = i.checked_sub(1) {
                    let p = &nodes[prev];
                    if p.token.token_type == TokenType::Name
                        && !n.prefix.is_empty()
                        && !n.prefix.contains('\n')
                    {
                        let (line, col) = line_index.offset_to_line_col(n.token.spos);
                        violations.push(Violation {
                            line,
                            col,
                            rule_id: self.id(),
                            message: format!(
                                "No space allowed between function '{}' and '('",
                                p.value
                            ),
                            severity: Severity::Error,
                        });
                    }
                }
            }
        }

        violations
    }

    fn fixes(&self, nodes: &[Node<'_>], _source: &str) -> Vec<Fix> {
        let mut fixes = Vec::new();

        for i in 0..nodes.len() {
            let n = &nodes[i];

            if n.token.token_type.is_whitespace_or_comment() || n.token.token_type.is_jinja() {
                continue;
            }

            // Remove space before comma.
            if n.token.token_type == TokenType::Comma {
                let prefix = &n.prefix;
                if !prefix.is_empty() && !prefix.contains('\n') {
                    fixes.push(Fix {
                        start: n.token.spos - prefix.len(),
                        end: n.token.spos,
                        replacement: String::new(),
                    });
                }
            }

            // Collapse multiple mid-line spaces to one.
            // Skip the func-paren case — that's handled below (and should be removed entirely).
            let prefix = &n.prefix;
            let prev_is_newline = i > 0 && nodes[i - 1].token.token_type == TokenType::Newline;
            let is_func_paren = n.token.token_type == TokenType::BracketOpen
                && n.value == "("
                && i > 0
                && nodes[i - 1].token.token_type == TokenType::Name;
            if !prefix.contains('\n') && prefix.len() > 1 && !prev_is_newline && !is_func_paren {
                fixes.push(Fix {
                    start: n.token.spos - prefix.len(),
                    end: n.token.spos,
                    replacement: " ".to_string(),
                });
            }

            // Remove space between function name and `(`.
            if n.token.token_type == TokenType::BracketOpen && n.value == "(" {
                if let Some(prev) = i.checked_sub(1) {
                    let p = &nodes[prev];
                    if p.token.token_type == TokenType::Name
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
            }
        }

        fixes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(Spacing, sql)
    }

    #[test]
    fn test_clean() {
        assert!(violations("select a, b from t").is_empty());
    }

    #[test]
    fn test_space_before_comma() {
        let v = violations("select a , b from t");
        assert!(!v.is_empty());
        assert!(v.iter().any(|x| x.message.contains("before ','")));
    }

    #[test]
    fn test_double_space() {
        let v = violations("select  a from t");
        assert!(!v.is_empty());
    }

    #[test]
    fn test_function_space() {
        let v = violations("select count (id) from t");
        assert!(!v.is_empty());
    }
}
