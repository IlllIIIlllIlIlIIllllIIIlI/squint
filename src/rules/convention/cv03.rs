use crate::analysis::Clause;
use crate::config::TrailingCommaPolicy;
use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// CV03 — Control trailing commas in SELECT clauses.
///
/// Default policy: `forbid` (no trailing comma before FROM).
pub struct TrailingComma {
    pub policy: TrailingCommaPolicy,
}

impl TrailingComma {
    pub fn new(policy: TrailingCommaPolicy) -> Self {
        TrailingComma { policy }
    }
}

impl Rule for TrailingComma {
    fn id(&self) -> &'static str {
        "CV03"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let clauses = ctx.clauses;
        let mut violations = Vec::new();

        // A "trailing comma" is a comma not followed by any real column token before FROM.
        // Strategy: track the last token seen — if the last non-whitespace token in the
        // SELECT clause (before FROM) is a comma, that's trailing.
        let mut last_meaningful: Option<(usize, bool)> = None; // (idx, is_comma)
        let mut in_select = false;

        for i in 0..nodes.len() {
            if nodes[i].bracket_depth != 0 {
                continue;
            }
            let is_ws = matches!(
                nodes[i].token.token_type,
                TokenType::Newline | TokenType::Comment
            );

            match &clauses[i] {
                Clause::Select => {
                    in_select = true;
                    // Skip the SELECT keyword itself and DISTINCT/ALL
                    if nodes[i].token.token_type == TokenType::StatementStart {
                        continue;
                    }
                    if !is_ws {
                        let is_comma = nodes[i].token.token_type == TokenType::Comma;
                        last_meaningful = Some((i, is_comma));
                    }
                }
                Clause::From if in_select => {
                    // Check what the last meaningful SELECT-clause token was
                    if let Some((comma_idx, true)) = last_meaningful {
                        if self.policy == TrailingCommaPolicy::Forbid {
                            let (line, col) =
                                line_index.offset_to_line_col(nodes[comma_idx].token.spos);
                            violations.push(Violation {
                                line,
                                col,
                                rule_id: self.id(),
                                message: "Trailing comma in SELECT clause is not allowed"
                                    .to_string(),
                                severity: Severity::Error,
                            });
                        }
                    } else if self.policy == TrailingCommaPolicy::Require {
                        if let Some((_, false)) = last_meaningful {
                            let (line, col) = line_index.offset_to_line_col(nodes[i].token.spos);
                            violations.push(Violation {
                                line,
                                col,
                                rule_id: self.id(),
                                message: "SELECT clause must end with a trailing comma".to_string(),
                                severity: Severity::Error,
                            });
                        }
                    }
                    in_select = false;
                    last_meaningful = None;
                }
                _ => {
                    if clauses[i] != Clause::Select {
                        in_select = false;
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
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::rules::LineIndex;

    fn violations(sql: &str, policy: TrailingCommaPolicy) -> Vec<Violation> {
        let nodes = Parser::new(Lexer::new(sql).tokenize()).parse();
        let line_index = LineIndex::new(sql);
        let clauses = crate::analysis::clause_map(&nodes);
        let ctx = LintContext {
            nodes: &nodes,
            source: sql,
            line_index: &line_index,
            clauses: &clauses,
        };
        TrailingComma::new(policy).check(&ctx)
    }

    #[test]
    fn test_forbid_no_trailing_ok() {
        assert!(violations("select a, b from t", TrailingCommaPolicy::Forbid).is_empty());
    }

    #[test]
    fn test_forbid_trailing_violation() {
        let v = violations("select a, b, from t", TrailingCommaPolicy::Forbid);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_require_trailing_ok() {
        assert!(violations("select a, b, from t", TrailingCommaPolicy::Require).is_empty());
    }
}
