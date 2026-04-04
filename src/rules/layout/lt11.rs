use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// LT11 — Set operators (UNION, INTERSECT, EXCEPT) must be preceded and
/// followed by a newline (i.e., on their own line).
pub struct SetOperators;

impl Rule for SetOperators {
    fn id(&self) -> &'static str {
        "LT11"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();

        for i in 0..nodes.len() {
            let n = &nodes[i];
            if n.token.token_type != TokenType::SetOperator {
                continue;
            }

            // Newlines are separate tokens; check if previous token is Newline or prefix has \n
            let preceded_by_newline = n.prefix.contains('\n')
                || (i > 0 && nodes[i - 1].token.token_type == TokenType::Newline);

            // After UNION [ALL|DISTINCT], the next real token (SELECT) must be on a new line.
            // Use index-based iteration to avoid the O(n) ptr::eq position search.
            let next_has_newline = {
                let mut j = i + 1;
                // Find the first non-whitespace token after the set operator.
                while j < nodes.len()
                    && matches!(
                        nodes[j].token.token_type,
                        TokenType::Newline | TokenType::Comment
                    )
                {
                    j += 1;
                }
                // If it's ALL/DISTINCT, skip it and find the next non-whitespace token.
                if j < nodes.len()
                    && (nodes[j].value.eq_ignore_ascii_case("all")
                        || nodes[j].value.eq_ignore_ascii_case("distinct"))
                {
                    j += 1;
                    while j < nodes.len()
                        && matches!(
                            nodes[j].token.token_type,
                            TokenType::Newline | TokenType::Comment
                        )
                    {
                        j += 1;
                    }
                }
                // The token at j must be on a new line.
                j >= nodes.len()
                    || nodes[j].prefix.contains('\n')
                    || (j > 0 && nodes[j - 1].token.token_type == TokenType::Newline)
            };

            if !preceded_by_newline || !next_has_newline {
                let (line, col) = line_index.offset_to_line_col(n.token.spos);
                violations.push(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!(
                        "'{}' must be on its own line (preceded and followed by a newline)",
                        n.value.to_uppercase()
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
        crate::rules::check_rule(SetOperators, sql)
    }

    #[test]
    fn test_ok() {
        let sql = "select a from t\nunion all\nselect a from u";
        assert!(violations(sql).is_empty());
    }

    #[test]
    fn test_inline_violation() {
        let v = violations("select a from t union all select a from u");
        assert!(!v.is_empty());
    }
}
