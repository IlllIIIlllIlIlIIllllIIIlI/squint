use crate::analysis::table_aliases_with_clauses;
use crate::rules::{LintContext, Rule, Severity, Violation};
use std::collections::HashMap;

/// AL04 — Table aliases must be unique within a query.
pub struct UniqueTableAliases;

impl Rule for UniqueTableAliases {
    fn id(&self) -> &'static str {
        "AL04"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut violations = Vec::new();

        for alias in table_aliases_with_clauses(nodes, ctx.clauses) {
            let key = alias.alias.to_lowercase();
            if let Some(first_idx) = seen.get(&key) {
                let (line, col) = line_index.offset_to_line_col(nodes[alias.alias_idx].token.spos);
                violations.push(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!(
                        "Table alias '{}' is already used (first defined at node {})",
                        alias.alias, first_idx
                    ),
                    severity: Severity::Error,
                });
            } else {
                seen.insert(key, alias.alias_idx);
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(UniqueTableAliases, sql)
    }

    #[test]
    fn test_unique_aliases_ok() {
        assert!(violations("select * from foo as a join bar as b on a.id = b.id").is_empty());
    }

    #[test]
    fn test_duplicate_alias() {
        let v = violations("select * from foo as t join bar as t on t.id = t.id");
        assert!(!v.is_empty());
    }
}
