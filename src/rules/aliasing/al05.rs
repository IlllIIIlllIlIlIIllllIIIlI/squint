use crate::analysis::table_aliases_with_clauses;
use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;
use std::collections::HashMap;

/// AL05 — Table aliases that are defined but never referenced should be removed.
pub struct UnusedAliases;

impl Rule for UnusedAliases {
    fn id(&self) -> &'static str {
        "AL05"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let aliases = table_aliases_with_clauses(nodes, ctx.clauses);
        if aliases.is_empty() {
            return vec![];
        }

        // Count occurrences of each lowercased Name token value in one O(n) pass.
        // alias.alias is already lowercased, so map keys match directly.
        let mut name_counts: HashMap<String, usize> = HashMap::new();
        for n in nodes {
            if n.token.token_type == TokenType::Name {
                *name_counts.entry(n.value.to_lowercase()).or_insert(0) += 1;
            }
        }

        let mut violations = Vec::new();
        for alias in &aliases {
            let def_is_name = nodes[alias.alias_idx].token.token_type == TokenType::Name;
            // O(1) lookup: alias.alias is already lowercased.
            let count = name_counts.get(alias.alias.as_str()).copied().unwrap_or(0);
            // If the definition is itself a Name token it contributes 1 to count.
            // Referenced means the name appears somewhere other than the definition.
            let referenced = if def_is_name { count > 1 } else { count > 0 };

            if !referenced {
                let (line, col) = line_index.offset_to_line_col(nodes[alias.alias_idx].token.spos);
                violations.push(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!("Table alias '{}' is defined but never used", alias.alias),
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
        crate::rules::check_rule(UnusedAliases, sql)
    }

    #[test]
    fn test_used_alias_ok() {
        assert!(violations("select a.id from foo as a").is_empty());
    }

    #[test]
    fn test_unused_alias() {
        let v = violations("select id from foo as unused_alias");
        assert!(!v.is_empty());
    }
}
