use crate::rules::{LintContext, Rule, Severity, Violation};

/// LT05 — Lines must not exceed the configured maximum length.
pub struct LineLength {
    pub max_line_length: usize,
}

impl LineLength {
    pub fn new(max_line_length: usize) -> Self {
        LineLength { max_line_length }
    }
}

impl Rule for LineLength {
    fn id(&self) -> &'static str {
        "LT05"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (_nodes, source, _line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        source
            .lines()
            .enumerate()
            .filter(|(_, line)| line.len() > self.max_line_length)
            .map(|(i, line)| Violation {
                line: i + 1,
                col: self.max_line_length + 1,
                rule_id: self.id(),
                message: format!(
                    "Line too long ({} > {} characters)",
                    line.len(),
                    self.max_line_length
                ),
                severity: Severity::Error,
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

    fn violations(sql: &str, max: usize) -> Vec<Violation> {
        let tokens = Lexer::new(sql).tokenize();
        let nodes = Parser::new(tokens).parse();
        let line_index = LineIndex::new(sql);
        let clauses = crate::analysis::clause_map(&nodes);
        let ctx = LintContext {
            nodes: &nodes,
            source: sql,
            line_index: &line_index,
            clauses: &clauses,
        };
        LineLength::new(max).check(&ctx)
    }

    #[test]
    fn test_no_violation_within_limit() {
        assert!(violations("select id from t", 120).is_empty());
    }

    #[test]
    fn test_violation_over_limit() {
        let long = "select ".to_string() + &"a".repeat(120);
        let v = violations(&long, 120);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].line, 1);
        assert_eq!(v[0].col, 121);
        assert_eq!(v[0].rule_id, "LT05");
    }

    #[test]
    fn test_exact_limit_is_ok() {
        let exact = "a".repeat(120);
        assert!(violations(&exact, 120).is_empty());
    }

    #[test]
    fn test_multiline_reports_correct_line() {
        let sql = "select id from t\n".to_string() + &"a".repeat(121);
        let v = violations(&sql, 120);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].line, 2);
    }

    #[test]
    fn test_custom_limit() {
        let v = violations("select id from t", 10);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].col, 11);
    }
}
