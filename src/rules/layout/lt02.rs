use crate::rules::{LintContext, Rule, Severity, Violation};

/// LT02 — Indentation must use spaces (no tabs), and indented lines must
/// use a consistent multiple of 4 spaces.
pub struct Indent;

impl Rule for Indent {
    fn id(&self) -> &'static str {
        "LT02"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (_nodes, source, _line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();

        for (line_idx, line) in source.lines().enumerate() {
            let line_num = line_idx + 1;

            // Count leading whitespace characters
            let leading: &str = {
                let rest = line.trim_start_matches([' ', '\t']);
                &line[..line.len() - rest.len()]
            };

            if leading.is_empty() {
                continue;
            }

            // Tabs are not allowed
            if leading.contains('\t') {
                violations.push(Violation {
                    line: line_num,
                    col: 1,
                    rule_id: self.id(),
                    message: "Indentation must use spaces, not tabs".to_string(),
                    severity: Severity::Error,
                });
                continue;
            }

            // Indentation must be a multiple of 4 spaces
            if !leading.len().is_multiple_of(4) {
                violations.push(Violation {
                    line: line_num,
                    col: 1,
                    rule_id: self.id(),
                    message: format!(
                        "Indentation of {} spaces is not a multiple of 4",
                        leading.len()
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
        crate::rules::check_rule(Indent, sql)
    }

    #[test]
    fn test_clean() {
        assert!(violations("select\n    id\nfrom t").is_empty());
    }

    #[test]
    fn test_tab_violation() {
        let v = violations("select\n\tid\nfrom t");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("tabs"));
    }

    #[test]
    fn test_odd_indent_violation() {
        let v = violations("select\n   id\nfrom t");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("multiple of 4"));
    }

    #[test]
    fn test_eight_spaces_ok() {
        assert!(violations("select\n        id\nfrom t").is_empty());
    }
}
