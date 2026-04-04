use crate::analysis::Clause;
use crate::node::Node;

pub mod aliasing;
pub mod ambiguous;
pub mod capitalisation;
pub mod convention;
pub mod jinja;
pub mod layout;
pub mod references;
pub mod structure;

/// A text replacement to apply to the source when auto-fixing.
pub struct Fix {
    /// Byte offset of the start of the range to replace.
    pub start: usize,
    /// Byte offset of the end of the range to replace (exclusive).
    pub end: usize,
    /// Text to substitute in place of `source[start..end]`.
    pub replacement: String,
}

/// Apply a set of fixes to `source`, processing right-to-left so earlier
/// byte offsets remain valid.  Overlapping fixes are skipped.
pub fn apply_fixes(source: &str, fixes: Vec<Fix>) -> String {
    let mut sorted = fixes;
    sorted.sort_unstable_by(|a, b| b.start.cmp(&a.start));
    let mut result = source.to_string();
    let mut blocked_before = usize::MAX;
    for fix in sorted {
        if fix.end > blocked_before {
            continue; // overlaps with an already-applied fix
        }
        result.replace_range(fix.start..fix.end, &fix.replacement);
        blocked_before = fix.start;
    }
    result
}

/// Severity of a lint violation — controls exit code and output labelling.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
        }
    }
}

/// A single lint violation.
#[derive(Debug, PartialEq, serde::Serialize)]
pub struct Violation {
    /// 1-based line number in the source file.
    pub line: usize,
    /// 1-based column number.
    pub col: usize,
    pub rule_id: &'static str,
    pub message: String,
    /// Severity — stamped by `lint_source` after collection, not by the rule itself.
    pub severity: Severity,
}

impl Default for Violation {
    fn default() -> Self {
        Violation {
            line: 0,
            col: 0,
            rule_id: "",
            message: String::new(),
            severity: Severity::Error,
        }
    }
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}: [{}] [{}] {}",
            self.line, self.col, self.severity, self.rule_id, self.message
        )
    }
}

/// Pre-computed line-start table for O(log n) byte-offset → (line, col) conversion.
///
/// Inspired by ruff's `SourceFile`: scan the source once on construction, storing
/// the byte offset of the first character of each line. Each lookup is then a
/// binary search (O(log lines)) instead of a linear byte scan (O(offset)).
pub struct LineIndex {
    /// `starts[i]` is the byte offset of the first character of line `i` (0-based).
    /// `starts[0]` is always 0.
    starts: Vec<usize>,
}

impl LineIndex {
    /// Build the index by scanning `source` for newlines — O(n) once.
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0usize];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        LineIndex { starts }
    }

    /// Convert a byte offset to 1-based (line, col) — O(log lines).
    pub fn offset_to_line_col(&self, offset: usize) -> (usize, usize) {
        // partition_point gives the first index where starts[i] > offset,
        // so the line containing `offset` is one before that.
        let line_idx = self
            .starts
            .partition_point(|&s| s <= offset)
            .saturating_sub(1);
        (line_idx + 1, offset - self.starts[line_idx] + 1)
    }
}

/// Shared context passed to every rule's `check()` call.
///
/// Built once per `lint_source` invocation and shared across all rules,
/// avoiding redundant re-computation of the clause map and line index.
pub struct LintContext<'a> {
    pub nodes: &'a [Node<'a>],
    pub source: &'a str,
    pub line_index: &'a LineIndex,
    /// Pre-computed clause assignment for every node — avoids repeated `clause_map` calls.
    pub clauses: &'a [Clause],
}

/// Build a `LintContext` from a SQL string and run `rule.check()` against it.
///
/// Used exclusively in per-rule unit tests to avoid repeating the same
/// six-line setup boilerplate in every `#[cfg(test)]` block.
#[cfg(test)]
pub fn check_rule<R: Rule>(rule: R, sql: &str) -> Vec<Violation> {
    use crate::analysis::clause_map;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    let nodes = Parser::new(Lexer::new(sql).tokenize()).parse();
    let line_index = LineIndex::new(sql);
    let clauses = clause_map(&nodes);
    let ctx = LintContext {
        nodes: &nodes,
        source: sql,
        line_index: &line_index,
        clauses: &clauses,
    };
    rule.check(&ctx)
}

/// All lint rules implement this trait.
pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation>;
    /// Return fixes that would resolve all violations for this rule.
    /// Default: no fixes (rule is not auto-fixable).
    fn fixes(&self, _nodes: &[Node<'_>], _source: &str) -> Vec<Fix> {
        vec![]
    }
    /// Default severity for violations from this rule.
    /// Can be overridden per-rule in `.sql-linter.toml`.
    fn severity(&self) -> Severity {
        Severity::Error
    }
}
