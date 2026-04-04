use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// LT10 — SELECT modifiers (DISTINCT, ALL) must be on the same line as SELECT.
pub struct SelectModifiers;

impl Rule for SelectModifiers {
    fn id(&self) -> &'static str {
        "LT10"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();

        for i in 0..nodes.len() {
            let n = &nodes[i];
            if n.token.token_type != TokenType::StatementStart
                || !n.value.eq_ignore_ascii_case("select")
            {
                continue;
            }

            // Scan forward for DISTINCT/ALL, skipping only spaces (not newlines)
            for j in i + 1..nodes.len() {
                let next = &nodes[j];
                if next.token.token_type == TokenType::Newline {
                    // Newline before DISTINCT/ALL — check if the token after the newline is the modifier
                    // (i.e., no non-whitespace between SELECT and the newline)
                    let between_clean = (i + 1..j).all(|k| {
                        matches!(
                            nodes[k].token.token_type,
                            TokenType::Newline | TokenType::Comment
                        )
                    });
                    if !between_clean {
                        break;
                    }
                    // Look at first meaningful token after newline(s)
                    let modifier = nodes[j + 1..].iter().find(|x| {
                        !matches!(x.token.token_type, TokenType::Newline | TokenType::Comment)
                    });
                    if let Some(m) = modifier {
                        if m.value.eq_ignore_ascii_case("distinct")
                            || m.value.eq_ignore_ascii_case("all")
                        {
                            let (line, col) = line_index.offset_to_line_col(m.token.spos);
                            violations.push(Violation {
                                line,
                                col,
                                rule_id: self.id(),
                                message: format!(
                                    "'{}' modifier must be on the same line as SELECT",
                                    m.value.to_uppercase()
                                ),
                                severity: Severity::Error,
                            });
                        }
                    }
                    break;
                }
                // Non-whitespace before a newline — modifier would be on same line (ok) or not present
                if !matches!(
                    next.token.token_type,
                    TokenType::Newline | TokenType::Comment
                ) {
                    break;
                }
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(SelectModifiers, sql)
    }

    #[test]
    fn test_ok() {
        assert!(violations("select distinct a from t").is_empty());
    }

    #[test]
    fn test_modifier_on_next_line() {
        let v = violations("select\ndistinct a from t");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("DISTINCT"));
    }
}
