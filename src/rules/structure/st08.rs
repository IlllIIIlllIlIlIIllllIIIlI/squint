use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// ST08 — `SELECT DISTINCT` must not be used when selecting only a literal or
/// a constant expression (the DISTINCT has no effect).
///
/// Also flags `COUNT(DISTINCT *)` which is invalid syntax in most dialects.
pub struct Distinct;

impl Rule for Distinct {
    fn id(&self) -> &'static str {
        "ST08"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();

        for i in 0..nodes.len() {
            let n = &nodes[i];

            // Flag `COUNT(DISTINCT *)` — DISTINCT on star inside COUNT
            if n.token.token_type == TokenType::Name && n.value.eq_ignore_ascii_case("count") {
                // Find ( after count
                let open = nodes[i + 1..]
                    .iter()
                    .position(|x| x.token.token_type == TokenType::BracketOpen && x.value == "(");
                if let Some(rel) = open {
                    let open_idx = i + 1 + rel;
                    let depth = nodes[open_idx].bracket_depth + 1;
                    // Find DISTINCT inside COUNT(
                    let distinct = nodes[open_idx + 1..].iter().find(|x| {
                        x.bracket_depth == depth
                            && x.token.token_type == TokenType::UntermintedKeyword
                            && x.value.eq_ignore_ascii_case("distinct")
                    });
                    if let Some(d) = distinct {
                        // Now check if next meaningful token is *
                        let star = nodes[open_idx + 1..].iter().find(|x| {
                            x.bracket_depth == depth
                                && !matches!(
                                    x.token.token_type,
                                    TokenType::Newline | TokenType::Comment
                                )
                                && !x.value.eq_ignore_ascii_case("distinct")
                        });
                        if let Some(s) = star {
                            if s.token.token_type == TokenType::Star {
                                let (line, col) = line_index.offset_to_line_col(d.token.spos);
                                violations.push(Violation {
                                    line, col,
                                    rule_id: self.id(),
                                    message: "COUNT(DISTINCT *) is invalid — use COUNT(*) or COUNT(DISTINCT col)".to_string(),
                                    severity: Severity::Error,
                                });
                            }
                        }
                    }
                }
            }

            // Flag SELECT DISTINCT <number_literal> — DISTINCT on a constant
            if n.token.token_type == TokenType::StatementStart
                && n.value.eq_ignore_ascii_case("select")
            {
                let distinct_idx = nodes[i + 1..].iter().position(|x| {
                    !matches!(x.token.token_type, TokenType::Newline | TokenType::Comment)
                        && x.value.eq_ignore_ascii_case("distinct")
                });
                if let Some(rel) = distinct_idx {
                    let dist_idx = i + 1 + rel;
                    // Find the first column expression
                    let first_col = nodes[dist_idx + 1..].iter().find(|x| {
                        !matches!(x.token.token_type, TokenType::Newline | TokenType::Comment)
                    });
                    if let Some(col) = first_col {
                        if col.token.token_type == TokenType::Number
                            || col.token.token_type == TokenType::Data
                        {
                            let (line, col_pos) =
                                line_index.offset_to_line_col(nodes[dist_idx].token.spos);
                            violations.push(Violation {
                                line,
                                col: col_pos,
                                rule_id: self.id(),
                                message: "DISTINCT on a constant expression has no effect"
                                    .to_string(),
                                severity: Severity::Error,
                            });
                        }
                    }
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
        crate::rules::check_rule(Distinct, sql)
    }

    #[test]
    fn test_ok() {
        assert!(violations("select distinct a from t").is_empty());
    }

    #[test]
    fn test_count_distinct_star() {
        let v = violations("select count(distinct *) from t");
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_distinct_constant() {
        let v = violations("select distinct 1");
        assert_eq!(v.len(), 1);
    }
}
