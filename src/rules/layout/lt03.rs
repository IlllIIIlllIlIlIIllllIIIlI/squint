use crate::node::Node;
use crate::rules::{Fix, LintContext, Rule, Severity, Violation};

/// LT03 — Lines must not have trailing whitespace (spaces or tabs before the newline).
///
/// This rule works directly on the source string rather than the token list,
/// since trailing whitespace appears in token prefixes and can span across
/// multiple tokens in ways that are easier to detect via string scanning.
pub struct TrailingWhitespace;

impl Rule for TrailingWhitespace {
    fn id(&self) -> &'static str {
        "LT03"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (_nodes, source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        let mut violations = Vec::new();

        for (start, end, _) in trailing_ws_ranges(source) {
            let (line, col) = line_index.offset_to_line_col(start);
            violations.push(Violation {
                line,
                col,
                rule_id: self.id(),
                message: "Trailing whitespace".to_string(),
                severity: Severity::Error,
            });
            let _ = end; // used in fixes
        }

        violations
    }

    fn fixes(&self, _nodes: &[Node<'_>], source: &str) -> Vec<Fix> {
        trailing_ws_ranges(source)
            .into_iter()
            .map(|(start, end, _)| Fix {
                start,
                end,
                replacement: String::new(),
            })
            .collect()
    }
}

/// Returns `(start_byte, end_byte, line_number)` for every run of trailing
/// whitespace on each line of `source`.
fn trailing_ws_ranges(source: &str) -> Vec<(usize, usize, usize)> {
    let mut result = Vec::new();
    let mut pos = 0;
    let mut line = 1usize;

    while pos <= source.len() {
        // Find the end of the current line (position of `\n`, or EOF).
        let line_end = source[pos..]
            .find('\n')
            .map(|i| pos + i)
            .unwrap_or(source.len());

        let line_content = &source[pos..line_end];
        let trimmed_len = line_content.trim_end_matches([' ', '\t']).len();

        if trimmed_len < line_content.len() {
            let ws_start = pos + trimmed_len;
            result.push((ws_start, line_end, line));
        }

        if line_end == source.len() {
            break;
        }
        pos = line_end + 1; // skip the `\n`
        line += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(TrailingWhitespace, sql)
    }

    fn fix(sql: &str) -> String {
        use crate::rules::apply_fixes;
        let rule = TrailingWhitespace;
        let nodes = {
            use crate::lexer::Lexer;
            use crate::parser::Parser;
            Parser::new(Lexer::new(sql).tokenize()).parse()
        };
        let fixes = rule.fixes(&nodes, sql);
        apply_fixes(sql, fixes)
    }

    #[test]
    fn test_clean_line_ok() {
        assert!(violations("select 1\n").is_empty());
    }

    #[test]
    fn test_trailing_spaces_flagged() {
        let v = violations("select 1   \nfrom t\n");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].line, 1);
        assert_eq!(v[0].rule_id, "LT03");
    }

    #[test]
    fn test_trailing_tab_flagged() {
        let v = violations("select 1\t\nfrom t\n");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].line, 1);
    }

    #[test]
    fn test_multiple_lines_flagged() {
        let v = violations("select 1   \nfrom t  \n");
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].line, 1);
        assert_eq!(v[1].line, 2);
    }

    #[test]
    fn test_fix_removes_trailing_spaces() {
        assert_eq!(fix("select 1   \nfrom t\n"), "select 1\nfrom t\n");
    }

    #[test]
    fn test_fix_multiple_lines() {
        assert_eq!(fix("select 1  \nfrom t  \n"), "select 1\nfrom t\n");
    }

    #[test]
    fn test_col_reported_at_first_trailing_space() {
        let v = violations("select 1   \n");
        assert_eq!(v[0].col, 9); // 1-based: "select 1" is 8 chars, space starts at col 9
    }
}
