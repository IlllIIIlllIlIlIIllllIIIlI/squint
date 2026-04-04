use crate::analysis::cte_definitions;
use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

/// ST03 — Remove CTEs that are defined but never referenced.
pub struct UnusedCte;

impl Rule for UnusedCte {
    fn id(&self) -> &'static str {
        "ST03"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let ctes = cte_definitions(nodes);
        if ctes.is_empty() {
            return vec![];
        }

        // Collect all Name/QuotedName token (index, value) pairs in one pass — O(n)
        let name_tokens: Vec<(usize, &str)> = nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                if matches!(n.token.token_type, TokenType::Name | TokenType::QuotedName) {
                    Some((i, n.value))
                } else {
                    None
                }
            })
            .collect();

        ctes.into_iter()
            .filter(|cte| {
                // Check for any reference outside the definition site — O(k) per CTE
                !name_tokens
                    .iter()
                    .any(|&(i, v)| i != cte.name_idx && v.eq_ignore_ascii_case(&cte.name))
            })
            .map(|cte| {
                let (line, col) = line_index.offset_to_line_col(nodes[cte.name_idx].token.spos);
                Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!("CTE '{}' is defined but never referenced", cte.name),
                    severity: Severity::Error,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(UnusedCte, sql)
    }

    #[test]
    fn test_used_cte_ok() {
        let sql = "with cte as (\n    select 1\n)\nselect * from cte";
        assert!(violations(sql).is_empty());
    }

    #[test]
    fn test_unused_cte() {
        let sql = "with cte as (\n    select 1\n)\nselect 1";
        let v = violations(sql);
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("cte"));
    }
}
