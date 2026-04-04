use crate::analysis::select_items_with_clauses;
use crate::rules::{LintContext, Rule, Severity, Violation};

/// AL02 — Column aliases must use explicit AS keyword.
///
/// `SELECT a alias` → violation; `SELECT a AS alias` → ok.
pub struct ImplicitAliases;

impl Rule for ImplicitAliases {
    fn id(&self) -> &'static str {
        "AL02"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        select_items_with_clauses(nodes, ctx.clauses)
            .into_iter()
            .filter(|item| item.alias.is_some() && item.as_kw.is_none())
            .map(|item| {
                let alias_idx = item.alias.unwrap();
                let (line, col) = line_index.offset_to_line_col(nodes[alias_idx].token.spos);
                Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!(
                        "Column alias '{}' must use explicit AS keyword",
                        nodes[alias_idx].value
                    ),
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
        crate::rules::check_rule(ImplicitAliases, sql)
    }

    #[test]
    fn test_explicit_alias_ok() {
        assert!(violations("select a as alias_col from t").is_empty());
    }

    #[test]
    fn test_implicit_alias_violation() {
        let v = violations("select a alias_col from t");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("alias_col"));
    }

    #[test]
    fn test_no_alias_ok() {
        assert!(violations("select a, b from t").is_empty());
    }
}
