use crate::analysis::Clause;
use crate::config::GroupByStyle;

const GROUP_BY_KEYWORDS: &[&str] = &[
    "group", "order", "by", "asc", "desc", "nulls", "first", "last",
];
use crate::node::Node;
use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// AM06 — GROUP BY and ORDER BY must use consistent column references
/// (either all explicit names or all positional numbers).
pub struct ColumnReferences {
    pub style: GroupByStyle,
}

impl ColumnReferences {
    pub fn new(style: GroupByStyle) -> Self {
        ColumnReferences { style }
    }
}

impl Rule for ColumnReferences {
    fn id(&self) -> &'static str {
        "AM06"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let clauses = ctx.clauses;
        let mut violations = Vec::new();

        for (i, n) in nodes.iter().enumerate() {
            if n.bracket_depth != 0 {
                continue;
            }
            if !matches!(clauses[i], Clause::GroupBy | Clause::OrderBy) {
                continue;
            }
            // Skip the GROUP/ORDER BY keyword nodes themselves
            if GROUP_BY_KEYWORDS
                .iter()
                .any(|k| n.value.eq_ignore_ascii_case(k))
            {
                continue;
            }
            if n.token.token_type == TokenType::Comma
                || n.token.token_type == TokenType::Newline
                || n.token.token_type == TokenType::Comment
            {
                continue;
            }

            let is_positional = n.token.token_type == TokenType::Number;

            match self.style {
                GroupByStyle::Explicit if is_positional => {
                    let (line, col) = line_index.offset_to_line_col(n.token.spos);
                    violations.push(Violation {
                        line, col,
                        rule_id: self.id(),
                        message: format!(
                            "Positional reference '{}' in GROUP/ORDER BY — use explicit column name",
                            n.value
                        ),
                        severity: Severity::Error,
                    });
                }
                GroupByStyle::Implicit if !is_positional => {
                    let (line, col) = line_index.offset_to_line_col(n.token.spos);
                    violations.push(Violation {
                        line,
                        col,
                        rule_id: self.id(),
                        message: format!(
                            "Explicit reference '{}' in GROUP/ORDER BY — use positional number",
                            n.value
                        ),
                        severity: Severity::Error,
                    });
                }
                GroupByStyle::Consistent => {
                    // Handled below — collect all and check consistency
                }
                _ => {}
            }
        }

        // Consistent mode: check that each GROUP/ORDER BY clause uses only one style
        if self.style == GroupByStyle::Consistent {
            violations.extend(check_consistent(
                nodes,
                clauses,
                source,
                line_index,
                self.id(),
            ));
        }

        violations
    }
}

fn check_consistent(
    nodes: &[Node<'_>],
    clauses: &[Clause],
    _source: &str,
    line_index: &crate::rules::LineIndex,
    rule_id: &'static str,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    for target_clause in [Clause::GroupBy, Clause::OrderBy] {
        let items: Vec<(usize, bool)> = nodes
            .iter()
            .enumerate()
            .filter(|(i, n)| {
                clauses[*i] == target_clause
                    && n.bracket_depth == 0
                    && !matches!(
                        n.token.token_type,
                        TokenType::Comma | TokenType::Newline | TokenType::Comment
                    )
                    && !GROUP_BY_KEYWORDS
                        .iter()
                        .any(|k| n.value.eq_ignore_ascii_case(k))
            })
            .map(|(i, n)| (i, n.token.token_type == TokenType::Number))
            .collect();

        if items.is_empty() {
            continue;
        }
        let first_is_positional = items[0].1;
        for (i, is_positional) in &items[1..] {
            if *is_positional != first_is_positional {
                let (line, col) = line_index.offset_to_line_col(nodes[*i].token.spos);
                violations.push(Violation {
                    line,
                    col,
                    rule_id,
                    message: "GROUP/ORDER BY mixes positional and explicit column references"
                        .to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::rules::LineIndex;

    fn violations(sql: &str, style: GroupByStyle) -> Vec<Violation> {
        let nodes = Parser::new(Lexer::new(sql).tokenize()).parse();
        let line_index = LineIndex::new(sql);
        let clauses = crate::analysis::clause_map(&nodes);
        let ctx = LintContext {
            nodes: &nodes,
            source: sql,
            line_index: &line_index,
            clauses: &clauses,
        };
        ColumnReferences::new(style).check(&ctx)
    }

    #[test]
    fn test_explicit_ok() {
        assert!(violations("select a, b from t group by a, b", GroupByStyle::Explicit).is_empty());
    }

    #[test]
    fn test_explicit_rejects_positional() {
        let v = violations("select a, b from t group by 1, 2", GroupByStyle::Explicit);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn test_implicit_ok() {
        assert!(violations("select a, b from t group by 1, 2", GroupByStyle::Implicit).is_empty());
    }

    #[test]
    fn test_consistent_mixed() {
        let v = violations("select a, b from t group by a, 2", GroupByStyle::Consistent);
        assert!(!v.is_empty());
    }
}
