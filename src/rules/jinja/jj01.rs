use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;
use regex::Regex;
use std::sync::OnceLock;

/// JJ01 — Jinja template tags must have single-space padding inside.
///
/// `{{x}}` → `{{ x }}`
/// `{%if x%}` → `{% if x %}`
pub struct JinjaPadding;

fn bad_expr_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Matches {{ without a space, or }} without a space before it
    RE.get_or_init(|| Regex::new(r"\{\{[^ ]|[^ ]\}\}").unwrap())
}

fn bad_stmt_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Matches {% without space or %} without space before
    RE.get_or_init(|| Regex::new(r"\{%-?[^ ]|[^ ]-?%\}").unwrap())
}

impl Rule for JinjaPadding {
    fn id(&self) -> &'static str {
        "JJ01"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();

        for n in nodes {
            let is_expr = n.token.token_type == TokenType::JinjaExpression;
            let is_stmt = matches!(
                n.token.token_type,
                TokenType::JinjaStatement
                    | TokenType::JinjaBlockStart
                    | TokenType::JinjaBlockKeyword
                    | TokenType::JinjaBlockEnd
            );

            if !is_expr && !is_stmt {
                continue;
            }

            let re = if is_expr {
                bad_expr_re()
            } else {
                bad_stmt_re()
            };

            if re.is_match(n.value) {
                let (line, col) = line_index.offset_to_line_col(n.token.spos);
                violations.push(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!(
                        "Jinja tag '{}' must have single-space padding inside delimiters",
                        n.value.trim()
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
        crate::rules::check_rule(JinjaPadding, sql)
    }

    #[test]
    fn test_ok() {
        assert!(violations("select {{ my_col }} from t").is_empty());
    }

    #[test]
    fn test_no_space_expr() {
        let v = violations("select {{my_col}} from t");
        assert!(!v.is_empty());
    }

    #[test]
    fn test_block_ok() {
        assert!(violations("{% if condition %}select 1{% endif %}").is_empty());
    }

    #[test]
    fn test_block_no_space() {
        let v = violations("{%if condition%}select 1{%endif%}");
        assert!(!v.is_empty());
    }
}
