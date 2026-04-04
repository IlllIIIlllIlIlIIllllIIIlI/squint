use crate::analysis::table_aliases_with_clauses;
use crate::rules::{LintContext, Rule, Severity, Violation};

/// AL06 — Table alias length must be within configured bounds.
pub struct AliasLength {
    pub min: usize,
    /// 0 = unlimited
    pub max: usize,
}

impl AliasLength {
    pub fn new(min: usize, max: usize) -> Self {
        AliasLength { min, max }
    }
}

impl Rule for AliasLength {
    fn id(&self) -> &'static str {
        "AL06"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        table_aliases_with_clauses(nodes, ctx.clauses)
            .into_iter()
            .filter_map(|alias| {
                let len = alias.alias.chars().count();
                let too_short = len < self.min;
                let too_long = self.max > 0 && len > self.max;
                if !too_short && !too_long {
                    return None;
                }
                let (line, col) = line_index.offset_to_line_col(nodes[alias.alias_idx].token.spos);
                let msg = if too_short {
                    format!(
                        "Alias '{}' is too short ({} < {} characters)",
                        alias.alias, len, self.min
                    )
                } else {
                    format!(
                        "Alias '{}' is too long ({} > {} characters)",
                        alias.alias, len, self.max
                    )
                };
                Some(Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: msg,
                    severity: Severity::Error,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::rules::LineIndex;

    fn violations(sql: &str, min: usize, max: usize) -> Vec<Violation> {
        let nodes = Parser::new(Lexer::new(sql).tokenize()).parse();
        let line_index = LineIndex::new(sql);
        let clauses = crate::analysis::clause_map(&nodes);
        let ctx = LintContext {
            nodes: &nodes,
            source: sql,
            line_index: &line_index,
            clauses: &clauses,
        };
        AliasLength::new(min, max).check(&ctx)
    }

    #[test]
    fn test_ok() {
        assert!(violations("select * from foo as bar", 2, 10).is_empty());
    }

    #[test]
    fn test_too_short() {
        let v = violations("select * from foo as a", 2, 0);
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("too short"));
    }

    #[test]
    fn test_too_long() {
        let v = violations("select * from foo as very_long_alias", 1, 5);
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("too long"));
    }
}
