use crate::analysis::select_items_with_clauses;
use crate::rules::{LintContext, Rule, Severity, Violation};
use std::collections::HashMap;

/// AL08 — Column aliases within a SELECT clause must be unique (case-insensitive).
pub struct UniqueColumnAliases;

impl Rule for UniqueColumnAliases {
    fn id(&self) -> &'static str {
        "AL08"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut violations = Vec::new();

        for item in select_items_with_clauses(nodes, ctx.clauses) {
            let Some(alias_idx) = item.alias else {
                continue;
            };
            let key = nodes[alias_idx].value.to_lowercase();
            if let Some(&first) = seen.get(&key) {
                let (line, col) = line_index.offset_to_line_col(nodes[alias_idx].token.spos);
                violations.push(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!(
                        "Column alias '{}' is already used (first at node {})",
                        nodes[alias_idx].value, first
                    ),
                    severity: Severity::Error,
                });
            } else {
                seen.insert(key, alias_idx);
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(UniqueColumnAliases, sql)
    }

    #[test]
    fn test_ok() {
        assert!(violations("select a as x, b as y from t").is_empty());
    }

    #[test]
    fn test_duplicate() {
        let v = violations("select a as foo, b as foo from t");
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_case_insensitive_duplicate() {
        let v = violations("select a as foo, b as FOO from t");
        assert!(!v.is_empty());
    }
}
