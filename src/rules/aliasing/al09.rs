use crate::analysis::select_items_with_clauses;
use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// AL09 — Column must not be aliased to itself (`col AS col`).
pub struct SelfAliases;

impl Rule for SelfAliases {
    fn id(&self) -> &'static str {
        "AL09"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();

        for item in select_items_with_clauses(nodes, ctx.clauses) {
            let (Some(as_idx), Some(alias_idx)) = (item.as_kw, item.alias) else {
                continue;
            };

            // The column name is the first meaningful token in the item (if it's a plain Name)
            let col_node = &nodes[item.start];
            if col_node.token.token_type != TokenType::Name
                && col_node.token.token_type != TokenType::QuotedName
            {
                continue;
            }

            // Check there's nothing between start and AS that would make it an expression
            let only_name = (item.start + 1..as_idx).all(|i| {
                matches!(
                    nodes[i].token.token_type,
                    TokenType::Newline | TokenType::Comment
                )
            });

            if only_name
                && nodes[item.start].value.to_lowercase() == nodes[alias_idx].value.to_lowercase()
            {
                let (line, col) = line_index.offset_to_line_col(nodes[alias_idx].token.spos);
                violations.push(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!(
                        "Column '{}' is aliased to itself — remove the alias",
                        nodes[item.start].value
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
        crate::rules::check_rule(SelfAliases, sql)
    }

    #[test]
    fn test_ok() {
        assert!(violations("select a as b from t").is_empty());
    }

    #[test]
    fn test_self_alias() {
        let v = violations("select col as col from t");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("col"));
    }

    #[test]
    fn test_case_insensitive_self_alias() {
        let v = violations("select Col as col from t");
        assert_eq!(v.len(), 1);
    }
}
