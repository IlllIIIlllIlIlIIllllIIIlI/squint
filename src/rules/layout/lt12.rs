use crate::node::Node;
use crate::rules::{Fix, LintContext, Rule, Severity, Violation};

/// LT12 — Files must end with a single trailing newline.
pub struct EndOfFile;

impl Rule for EndOfFile {
    fn id(&self) -> &'static str {
        "LT12"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (_nodes, source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        if source.is_empty() {
            return vec![];
        }

        let trailing_newlines = source.chars().rev().take_while(|&c| c == '\n').count();

        if trailing_newlines == 0 {
            let (line, col) = line_index.offset_to_line_col(source.len());
            return vec![Violation {
                line,
                col,
                rule_id: self.id(),
                message: "File must end with a single trailing newline".to_string(),
                severity: Severity::Error,
            }];
        }

        if trailing_newlines > 1 {
            let (line, col) = line_index.offset_to_line_col(source.len());
            return vec![Violation {
                line,
                col,
                rule_id: self.id(),
                message: format!(
                    "File has {} trailing newlines; expected exactly 1",
                    trailing_newlines
                ),
                severity: Severity::Error,
            }];
        }

        vec![]
    }

    fn fixes(&self, _nodes: &[Node<'_>], source: &str) -> Vec<Fix> {
        if source.is_empty() {
            return vec![];
        }
        let trailing_newlines = source.chars().rev().take_while(|&c| c == '\n').count();
        if trailing_newlines == 0 {
            // Append a newline at end of file.
            return vec![Fix {
                start: source.len(),
                end: source.len(),
                replacement: "\n".to_string(),
            }];
        }
        if trailing_newlines > 1 {
            // Remove all but the first trailing newline.
            let keep_until = source.len() - trailing_newlines + 1;
            return vec![Fix {
                start: keep_until,
                end: source.len(),
                replacement: String::new(),
            }];
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(EndOfFile, sql)
    }

    #[test]
    fn test_ok() {
        assert!(violations("select 1\n").is_empty());
    }

    #[test]
    fn test_no_newline() {
        let v = violations("select 1");
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_multiple_newlines() {
        let v = violations("select 1\n\n");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("2 trailing"));
    }
}
