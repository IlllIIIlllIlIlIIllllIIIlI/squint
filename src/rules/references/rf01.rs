use crate::analysis::{table_aliases_with_clauses, Clause};
use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// RF01 — Qualified column references (`alias.column`) must use an alias that
/// is defined in the FROM clause of the same query.
///
/// Note: This is a best-effort implementation. It handles simple single-level
/// queries and may produce false positives/negatives in complex cases
/// (subqueries, CTEs, USING clauses, etc.).
pub struct References;

impl Rule for References {
    fn id(&self) -> &'static str {
        "RF01"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let clauses = ctx.clauses;
        let aliases = table_aliases_with_clauses(nodes, clauses);

        // Collect known qualifiers: alias names + bare table names at top level
        let mut known: std::collections::HashSet<String> =
            aliases.iter().map(|a| a.alias.to_lowercase()).collect();

        // Also include bare table names from FROM clause (no alias)
        for (i, n) in nodes.iter().enumerate() {
            if clauses[i] == Clause::From
                && n.bracket_depth == 0
                && (n.token.token_type == TokenType::Name
                    || n.token.token_type == TokenType::QuotedName)
            {
                known.insert(n.value.to_lowercase());
            }
        }

        let mut violations = Vec::new();

        // Find `name.column` patterns and check `name` is known
        for i in 0..nodes.len() {
            let n = &nodes[i];
            if n.token.token_type != TokenType::Dot {
                continue;
            }
            // Token before the dot
            if i == 0 {
                continue;
            }
            let qualifier = &nodes[i - 1];
            if qualifier.token.token_type != TokenType::Name
                && qualifier.token.token_type != TokenType::QuotedName
            {
                continue;
            }
            // Only flag in non-FROM, non-WITH clauses (qualifiers in FROM are definitions)
            if matches!(clauses[i], Clause::From | Clause::With) {
                continue;
            }

            let q_lower = qualifier.value.to_lowercase();
            if !known.contains(&q_lower) {
                let (line, col) = line_index.offset_to_line_col(qualifier.token.spos);
                violations.push(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!(
                        "Qualifier '{}' is not defined in the FROM clause",
                        qualifier.value
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
        crate::rules::check_rule(References, sql)
    }

    #[test]
    fn test_ok() {
        assert!(violations("select a.id from foo as a").is_empty());
    }

    #[test]
    fn test_unknown_qualifier() {
        let v = violations("select x.id from foo as a");
        assert!(!v.is_empty());
        assert!(v[0].message.contains("x"));
    }

    #[test]
    fn test_bare_table_name_ok() {
        assert!(violations("select foo.id from foo").is_empty());
    }
}
