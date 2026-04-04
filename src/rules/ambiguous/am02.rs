use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// AM02 — UNION must be followed by DISTINCT or ALL to avoid ambiguity.
pub struct UnionDistinct;

impl Rule for UnionDistinct {
    fn id(&self) -> &'static str {
        "AM02"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();

        for i in 0..nodes.len() {
            let n = &nodes[i];
            if n.token.token_type != TokenType::SetOperator
                || !n.value.eq_ignore_ascii_case("union")
            {
                continue;
            }

            // Find next meaningful token after UNION
            let next = nodes[i + 1..]
                .iter()
                .find(|x| !matches!(x.token.token_type, TokenType::Newline | TokenType::Comment));

            let followed_by_qualifier = next.is_some_and(|x| {
                x.value.eq_ignore_ascii_case("all") || x.value.eq_ignore_ascii_case("distinct")
            });

            if !followed_by_qualifier {
                let (line, col) = line_index.offset_to_line_col(n.token.spos);
                violations.push(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: "UNION must be followed by DISTINCT or ALL".to_string(),
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
        crate::rules::check_rule(UnionDistinct, sql)
    }

    #[test]
    fn test_union_all_ok() {
        assert!(violations("select a from t union all select a from u").is_empty());
    }

    #[test]
    fn test_union_distinct_ok() {
        assert!(violations("select a from t union distinct select a from u").is_empty());
    }

    #[test]
    fn test_bare_union_violation() {
        let v = violations("select a from t union select a from u");
        assert_eq!(v.len(), 1);
    }
}
