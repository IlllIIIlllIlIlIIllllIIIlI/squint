use crate::analysis::{clause_map, compute_fmt_off_ranges, compute_noqa_lines};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::rules::{apply_fixes, LineIndex, LintContext, Rule, Violation};

pub fn lint_source(source: &str, rules: &[&dyn Rule]) -> Vec<Violation> {
    let tokens = Lexer::new(source).tokenize();
    let nodes = Parser::new(tokens).parse();
    let line_index = LineIndex::new(source);
    let clauses = clause_map(&nodes); // computed once, shared across all rules
    let fmt_off = compute_fmt_off_ranges(&nodes, source);
    let noqa = compute_noqa_lines(&nodes, source);
    let ctx = LintContext {
        nodes: &nodes,
        source,
        line_index: &line_index,
        clauses: &clauses,
    };
    let mut violations: Vec<Violation> = rules
        .iter()
        .flat_map(|r| {
            let sev = r.severity();
            r.check(&ctx).into_iter().map(move |mut v| {
                v.severity = sev;
                v
            })
        })
        .filter(|v| !fmt_off.is_line_off(v.line) && !noqa.suppresses(v.rule_id, v.line))
        .collect();
    // Sort by line then col for consistent output
    violations.sort_by_key(|v| (v.line, v.col));
    violations
}

/// Apply all auto-fixable rules to `source`, iterating until the source
/// stabilises or a maximum number of passes is reached.
/// Returns the (possibly modified) source string.
pub fn fix_source(source: &str, rules: &[&dyn Rule]) -> String {
    let mut current = source.to_string();
    for _ in 0..10 {
        let tokens = Lexer::new(&current).tokenize();
        let nodes = Parser::new(tokens).parse();
        let fmt_off = compute_fmt_off_ranges(&nodes, &current);
        let noqa = compute_noqa_lines(&nodes, &current);
        let fixes = rules
            .iter()
            .flat_map(|r| r.fixes(&nodes, &current))
            .filter(|fix| {
                !fmt_off.is_offset_off(&current, fix.start)
                    && !noqa.is_offset_suppressed_all(&current, fix.start)
            })
            .collect::<Vec<_>>();
        if fixes.is_empty() {
            break;
        }
        current = apply_fixes(&current, fixes);
    }
    current
}
