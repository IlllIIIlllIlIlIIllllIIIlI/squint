use crate::rules::{LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;
use std::collections::{HashMap, HashSet};

/// CV10 — Identifiers must use a consistent quoting style within a file.
///
/// If an identifier is used both quoted (`"col"`) and unquoted (`col`) in the
/// same file, the second occurrence is flagged. Only the first inconsistency
/// per identifier name is reported.
pub struct QuotingStyle;

impl Rule for QuotingStyle {
    fn id(&self) -> &'static str {
        "CV10"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let mut violations = Vec::new();
        // canonical name (lowercase, unquoted) → true if first occurrence was quoted
        let mut first_style: HashMap<String, bool> = HashMap::new();
        // names already flagged (report only first conflict per name)
        let mut flagged: HashSet<String> = HashSet::new();

        for node in ctx.nodes {
            let (canonical, is_quoted) = match node.token.token_type {
                TokenType::Name => (node.value.to_lowercase(), false),
                TokenType::QuotedName => (strip_quotes(node.value).to_lowercase(), true),
                _ => continue,
            };

            match first_style.get(&canonical) {
                None => {
                    first_style.insert(canonical, is_quoted);
                }
                Some(&first_was_quoted) => {
                    if first_was_quoted != is_quoted && !flagged.contains(&canonical) {
                        flagged.insert(canonical.clone());
                        let (line, col) = ctx.line_index.offset_to_line_col(node.token.spos);
                        let here = if is_quoted { "quoted" } else { "unquoted" };
                        let there = if is_quoted { "unquoted" } else { "quoted" };
                        violations.push(Violation {
                            line,
                            col,
                            rule_id: self.id(),
                            message: format!(
                                "Inconsistent quoting: identifier '{}' is {} here but {} elsewhere",
                                canonical, here, there
                            ),
                            severity: Severity::Error,
                        });
                    }
                }
            }
        }

        violations
    }
}

/// Strip surrounding quote characters from a quoted identifier token.
/// Handles double quotes (`"name"`), backticks (`` `name` ``), and brackets (`[name]`).
fn strip_quotes(s: &str) -> &str {
    if s.len() >= 2 {
        let bytes = s.as_bytes();
        let first = bytes[0];
        let last = bytes[s.len() - 1];
        if (first == b'"' && last == b'"')
            || (first == b'`' && last == b'`')
            || (first == b'[' && last == b']')
        {
            return &s[1..s.len() - 1];
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(QuotingStyle, sql)
    }

    #[test]
    fn test_all_unquoted_ok() {
        assert!(violations("select col from my_table where col = 1").is_empty());
    }

    #[test]
    fn test_all_quoted_ok() {
        assert!(violations(r#"select "col" from "my_table" where "col" = 1"#).is_empty());
    }

    #[test]
    fn test_mixed_quoting_flagged() {
        // 'col' first unquoted, then quoted
        let v = violations(r#"select col, "col" from t"#);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule_id, "CV10");
        assert!(v[0].message.contains("col"));
    }

    #[test]
    fn test_first_quoted_then_unquoted_flagged() {
        let v = violations(r#"select "col", col from t"#);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule_id, "CV10");
    }

    #[test]
    fn test_multiple_names_one_inconsistent() {
        let v = violations(r#"select a, "b", b from t"#);
        assert_eq!(v.len(), 1); // only 'b' is inconsistent
    }

    #[test]
    fn test_same_name_only_one_violation_per_name() {
        // 'col' appears quoted twice but was first seen unquoted — only one violation
        let v = violations(r#"select col, "col", "col" from t"#);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_backtick_quoting() {
        let v = violations("select col, `col` from t");
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("col"));
    }

    #[test]
    fn test_different_names_each_consistent_ok() {
        // 'a' always unquoted, 'b' always quoted — no mixing
        assert!(violations(r#"select a, "b" from t"#).is_empty());
    }

    #[test]
    fn test_case_insensitive_comparison() {
        // COL (unquoted) and "col" (quoted) are treated as the same identifier
        let v = violations(r#"select COL, "col" from t"#);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_violation_position() {
        // "select col, " is 12 chars; `"col"` starts at byte 12
        let v = violations(r#"select col, "col" from t"#);
        assert_eq!(v[0].line, 1);
        assert_eq!(v[0].col, 13);
    }
}
