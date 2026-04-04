use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// CV04 — Consistent row-counting syntax (`COUNT(*)` vs `COUNT(1)`).
///
/// Default: require `COUNT(*)`. Set `prefer_count_1 = true` to require `COUNT(1)`.
pub struct CountRows {
    pub prefer_count_1: bool,
}

impl CountRows {
    pub fn new(prefer_count_1: bool) -> Self {
        CountRows { prefer_count_1 }
    }
}

impl Rule for CountRows {
    fn id(&self) -> &'static str {
        "CV04"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();

        for i in 0..nodes.len() {
            // Look for `count` followed by `(`
            let n = &nodes[i];
            if n.token.token_type != TokenType::Name || n.value.to_lowercase() != "count" {
                continue;
            }
            let Some(open_idx) = nodes[i + 1..]
                .iter()
                .position(|x| x.token.token_type == TokenType::BracketOpen && x.value == "(")
            else {
                continue;
            };
            let open_idx = i + 1 + open_idx;

            // Find the first meaningful token inside the parens
            let inner = nodes[open_idx + 1..].iter().find(|x| {
                !matches!(x.token.token_type, TokenType::Newline | TokenType::Comment)
                    && x.bracket_depth == nodes[open_idx].bracket_depth + 1
            });

            let Some(inner_node) = inner else { continue };

            let is_star = inner_node.token.token_type == TokenType::Star;
            let is_one =
                inner_node.token.token_type == TokenType::Number && inner_node.value == "1";

            if self.prefer_count_1 && is_star {
                let (line, col) = line_index.offset_to_line_col(inner_node.token.spos);
                violations.push(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: "Use COUNT(1) instead of COUNT(*)".to_string(),
                    severity: Severity::Error,
                });
            } else if !self.prefer_count_1 && is_one {
                let (line, col) = line_index.offset_to_line_col(inner_node.token.spos);
                violations.push(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: "Use COUNT(*) instead of COUNT(1)".to_string(),
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
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::rules::LineIndex;

    fn violations(sql: &str, prefer_1: bool) -> Vec<Violation> {
        let nodes = Parser::new(Lexer::new(sql).tokenize()).parse();
        let line_index = LineIndex::new(sql);
        let clauses = crate::analysis::clause_map(&nodes);
        let ctx = LintContext {
            nodes: &nodes,
            source: sql,
            line_index: &line_index,
            clauses: &clauses,
        };
        CountRows::new(prefer_1).check(&ctx)
    }

    #[test]
    fn test_count_star_ok() {
        assert!(violations("select count(*) from t", false).is_empty());
    }

    #[test]
    fn test_count_1_violation() {
        let v = violations("select count(1) from t", false);
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("COUNT(*)"));
    }

    #[test]
    fn test_count_star_violation_when_prefer_1() {
        let v = violations("select count(*) from t", true);
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("COUNT(1)"));
    }
}
