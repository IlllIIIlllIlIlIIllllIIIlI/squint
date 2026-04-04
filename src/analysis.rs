/// Shared SQL structure analysis helpers used by multiple rules.
use crate::node::Node;
use crate::tokens::TokenType;
use std::collections::{HashMap, HashSet};

// ── fmt:off ranges ────────────────────────────────────────────────────────────

/// Tracks which source lines fall inside a `-- fmt: off` region.
/// Used by the linter to suppress violations and fixes in those regions.
pub struct FmtOffRanges {
    /// 1-based (start_line, end_line) inclusive ranges.
    ranges: Vec<(usize, usize)>,
}

impl FmtOffRanges {
    /// Returns true if `line` (1-based) is inside any fmt:off region.
    pub fn is_line_off(&self, line: usize) -> bool {
        self.ranges.iter().any(|&(s, e)| line >= s && line <= e)
    }

    /// Returns true if the byte offset (e.g. `fix.start`) falls in a fmt:off region.
    pub fn is_offset_off(&self, source: &str, byte_offset: usize) -> bool {
        if self.ranges.is_empty() {
            return false;
        }
        self.is_line_off(byte_to_line(source, byte_offset))
    }
}

/// Scan `nodes` and build the set of line ranges suppressed by `-- fmt: off` / `-- fmt: on`.
///
/// Three modes:
/// - **Inline** (`-- fmt: off` after SQL on the same line): suppresses that line only.
/// - **Block** (standalone `-- fmt: off` … `-- fmt: on`): suppresses the range between them.
/// - **To EOF** (standalone `-- fmt: off` with no matching `-- fmt: on`): suppresses from
///   that line to the end of the file.
pub fn compute_fmt_off_ranges(nodes: &[Node<'_>], source: &str) -> FmtOffRanges {
    // Fast path: skip the full node scan for files with no fmt markers (the common case).
    // `str::contains` uses SIMD-optimised byte search and is ~10 ns for typical files.
    if !source.contains("fmt:") {
        return FmtOffRanges { ranges: Vec::new() };
    }

    let mut ranges = Vec::new();

    for (i, node) in nodes.iter().enumerate() {
        if node.token.token_type != TokenType::FmtOff {
            continue;
        }

        // Standalone: the token is the first on its line.
        // Since newlines are their own tokens, a standalone `-- fmt: off` is
        // preceded by a Newline token (or is the very first token in the file).
        let is_standalone = i == 0 || nodes[i - 1].token.token_type == TokenType::Newline;

        let start_line = byte_to_line(source, node.token.spos);

        if is_standalone {
            // Block or to-EOF: find the next FmtOn token.
            let end_line = nodes[i + 1..]
                .iter()
                .find(|n| n.token.token_type == TokenType::FmtOn)
                .map(|n| byte_to_line(source, n.token.spos))
                .unwrap_or(usize::MAX); // no FmtOn → suppress to EOF
            ranges.push((start_line, end_line));
        } else {
            // Inline: suppress this line only.
            ranges.push((start_line, start_line));
        }
    }

    FmtOffRanges { ranges }
}

/// Convert a byte offset to a 1-based line number by counting preceding newlines.
fn byte_to_line(source: &str, pos: usize) -> usize {
    source[..pos.min(source.len())]
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
        + 1
}

// ── noqa per-line suppression ─────────────────────────────────────────────────

/// Tracks which source lines have `-- noqa` comments and which rules they suppress.
pub struct NoqaLines {
    /// 1-based line number → `None` (suppress all rules) or `Some(set of rule IDs)`.
    lines: HashMap<usize, Option<HashSet<String>>>,
}

impl NoqaLines {
    /// Returns true if `rule_id` is suppressed on `line` (1-based).
    pub fn suppresses(&self, rule_id: &str, line: usize) -> bool {
        match self.lines.get(&line) {
            None => false,
            Some(None) => true,                       // bare `-- noqa`
            Some(Some(ids)) => ids.contains(rule_id), // `-- noqa: CP01,LT05`
        }
    }

    /// Returns true if the byte offset falls on a line that suppresses all rules.
    /// Used by `fix_source` to filter out fixes on noqa lines.
    pub fn is_offset_suppressed_all(&self, source: &str, byte_offset: usize) -> bool {
        let line = byte_to_line(source, byte_offset);
        matches!(self.lines.get(&line), Some(None))
    }
}

/// Scan `nodes` and build the set of per-line noqa suppressions from `-- noqa` comments.
///
/// - `-- noqa` suppresses all rules on that line.
/// - `-- noqa: CP01,LT05` suppresses only the listed rules on that line.
/// - Rule IDs are normalised to uppercase.
pub fn compute_noqa_lines(nodes: &[Node<'_>], source: &str) -> NoqaLines {
    if !source.contains("noqa") {
        return NoqaLines {
            lines: HashMap::new(),
        };
    }
    let mut lines = HashMap::new();
    for node in nodes {
        if node.token.token_type != TokenType::Comment {
            continue;
        }
        if let Some(entry) = parse_noqa(node.value) {
            let line = byte_to_line(source, node.token.spos);
            lines.insert(line, entry);
        }
    }
    NoqaLines { lines }
}

/// Parse a comment token value into a noqa directive.
/// Returns `Some(None)` for `-- noqa`, `Some(Some(ids))` for `-- noqa: ID,ID`, `None` otherwise.
fn parse_noqa(s: &str) -> Option<Option<HashSet<String>>> {
    // Strip leading dashes and optional whitespace to get the comment body.
    let body = s.trim_start_matches('-').trim_start_matches([' ', '\t']);
    let rest = body.strip_prefix("noqa")?;
    let rest = rest.trim_start_matches([' ', '\t']);
    if rest.is_empty() {
        return Some(None); // bare `-- noqa` → suppress all
    }
    let rest = rest.strip_prefix(':')?;
    let ids: HashSet<String> = rest.split(',').map(|s| s.trim().to_uppercase()).collect();
    if ids.is_empty() {
        return Some(None);
    }
    Some(Some(ids))
}

// ── Clause tracking ───────────────────────────────────────────────────────────

/// High-level SQL clauses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Clause {
    With,
    Select,
    From,
    Where,
    GroupBy,
    OrderBy,
    Having,
    Limit,
    Set,
    Into,
    Values,
    Returning,
    Other,
}

/// For each node, compute which top-level clause it belongs to.
/// Nodes inside subqueries (bracket_depth > 0) inherit the clause of the
/// enclosing bracket unless they encounter a clause keyword at their own depth.
pub fn clause_map(nodes: &[Node<'_>]) -> Vec<Clause> {
    let mut result = vec![Clause::Other; nodes.len()];
    let mut stack: Vec<Clause> = vec![Clause::Other]; // stack per bracket depth

    for (i, node) in nodes.iter().enumerate() {
        let depth = node.bracket_depth;

        // Grow/shrink stack to match current depth
        while stack.len() <= depth {
            stack.push(stack.last().copied().unwrap_or(Clause::Other));
        }
        while stack.len() > depth + 1 {
            stack.pop();
        }

        // Update current clause if this is a clause-starting keyword at this depth
        if let Some(clause) = keyword_to_clause(node.value) {
            stack[depth] = clause;
        }

        result[i] = stack[depth];
    }

    result
}

fn keyword_to_clause(value: &str) -> Option<Clause> {
    if value.eq_ignore_ascii_case("with") {
        return Some(Clause::With);
    }
    if value.eq_ignore_ascii_case("select") {
        return Some(Clause::Select);
    }
    if value.eq_ignore_ascii_case("where") {
        return Some(Clause::Where);
    }
    if value.eq_ignore_ascii_case("group") {
        return Some(Clause::GroupBy);
    }
    if value.eq_ignore_ascii_case("order") {
        return Some(Clause::OrderBy);
    }
    if value.eq_ignore_ascii_case("having") {
        return Some(Clause::Having);
    }
    if value.eq_ignore_ascii_case("limit") {
        return Some(Clause::Limit);
    }
    if value.eq_ignore_ascii_case("set") {
        return Some(Clause::Set);
    }
    if value.eq_ignore_ascii_case("into") {
        return Some(Clause::Into);
    }
    if value.eq_ignore_ascii_case("values") {
        return Some(Clause::Values);
    }
    if value.eq_ignore_ascii_case("returning") {
        return Some(Clause::Returning);
    }
    if matches_any_ci(
        value,
        &[
            "from", "join", "inner", "left", "right", "full", "outer", "cross",
        ],
    ) {
        return Some(Clause::From);
    }
    None
}

#[inline(always)]
fn matches_any_ci(value: &str, list: &[&str]) -> bool {
    list.iter().any(|k| value.eq_ignore_ascii_case(k))
}

// ── Select items ─────────────────────────────────────────────────────────────

/// A single item in a SELECT clause (a column or expression).
#[derive(Debug)]
pub struct SelectItem {
    /// Index of the first non-whitespace node in this item.
    pub start: usize,
    /// Index of the last node in this item (inclusive, before the comma or FROM).
    pub end: usize,
    /// Index of the AS keyword node, if present.
    pub as_kw: Option<usize>,
    /// Index of the alias Name token, if present.
    pub alias: Option<usize>,
}

/// Parse all SELECT clause items using a pre-computed clause map.
/// Avoids recomputing `clause_map` when the caller already has one.
pub fn select_items_with_clauses(nodes: &[Node<'_>], clauses: &[Clause]) -> Vec<SelectItem> {
    let mut items: Vec<SelectItem> = Vec::new();
    let mut item_start: Option<usize> = None;

    for i in 0..nodes.len() {
        let n = &nodes[i];

        // Only look at top-level SELECT clause nodes
        if clauses[i] != Clause::Select || n.bracket_depth != 0 {
            // If we were building an item and we left the SELECT clause, close it
            if clauses[i] != Clause::Select {
                if let Some(start) = item_start.take() {
                    let item = build_item(nodes, start, i.saturating_sub(1));
                    items.push(item);
                }
            }
            continue;
        }

        // Skip the SELECT keyword itself and DISTINCT/ALL modifiers
        if n.token.token_type == TokenType::StatementStart {
            continue;
        }
        if (n.token.token_type == TokenType::UntermintedKeyword
            || n.token.token_type == TokenType::Name)
            && matches_any_ci(n.value, &["distinct", "all", "top"])
            && item_start.is_none()
        {
            continue;
        }
        // Skip the TOP N modifier (number after TOP)
        if item_start.is_none() && n.token.token_type == TokenType::Number {
            continue;
        }

        if n.token.token_type == TokenType::Comma {
            // Comma at depth 0 ends this item
            if let Some(start) = item_start.take() {
                let item = build_item(nodes, start, i - 1);
                items.push(item);
            }
        } else if n.token.token_type == TokenType::Newline
            || n.token.token_type == TokenType::Comment
        {
            // Ignore whitespace tokens for item tracking
        } else if item_start.is_none() {
            item_start = Some(i);
        }
    }

    // Close final item if SELECT was the last clause
    if let Some(start) = item_start {
        let end = nodes.len().saturating_sub(1);
        let item = build_item(nodes, start, end);
        items.push(item);
    }

    items
}

/// Parse all SELECT clause items in the node list.
/// Only considers top-level SELECT clauses (bracket_depth == 0).
#[allow(dead_code)]
pub fn select_items(nodes: &[Node<'_>]) -> Vec<SelectItem> {
    select_items_with_clauses(nodes, &clause_map(nodes))
}

fn build_item(nodes: &[Node<'_>], start: usize, end: usize) -> SelectItem {
    // Find the real end (skip trailing newlines/comments)
    let mut real_end = end;
    while real_end > start {
        let tt = &nodes[real_end].token.token_type;
        if *tt == TokenType::Newline || *tt == TokenType::Comment {
            real_end -= 1;
        } else {
            break;
        }
    }

    // Find AS keyword and alias
    let mut as_kw: Option<usize> = None;
    let mut alias: Option<usize> = None;

    for (rel, node) in nodes[start..=real_end].iter().enumerate() {
        if node.token.token_type == TokenType::UntermintedKeyword
            && node.value.eq_ignore_ascii_case("as")
        {
            as_kw = Some(start + rel);
        }
    }

    if let Some(as_j) = as_kw {
        // Alias is the first Name/QuotedName after AS
        alias = nodes[as_j + 1..=real_end]
            .iter()
            .position(|n| {
                n.token.token_type == TokenType::Name || n.token.token_type == TokenType::QuotedName
            })
            .map(|rel| as_j + 1 + rel);
    } else if real_end > start {
        // Possible implicit alias: last token is Name/QuotedName
        let last_tt = &nodes[real_end].token.token_type;
        if *last_tt == TokenType::Name || *last_tt == TokenType::QuotedName {
            // Check the second-to-last meaningful token is not an operator/keyword
            // (to avoid flagging `a + b` as having an implicit alias `b`)
            let prev = prev_non_ws(nodes, real_end);
            if let Some(p) = prev {
                let ptt = &nodes[p].token.token_type;
                if matches!(
                    ptt,
                    TokenType::Name
                        | TokenType::QuotedName
                        | TokenType::BracketClose
                        | TokenType::Number
                        | TokenType::Data
                ) {
                    alias = Some(real_end);
                }
            }
        }
    }

    SelectItem {
        start,
        end: real_end,
        as_kw,
        alias,
    }
}

/// Returns the index of the previous non-newline, non-comment node before `idx`.
pub fn prev_non_ws(nodes: &[Node<'_>], idx: usize) -> Option<usize> {
    let mut i = idx.checked_sub(1)?;
    loop {
        let tt = &nodes[i].token.token_type;
        if *tt != TokenType::Newline && *tt != TokenType::Comment {
            return Some(i);
        }
        i = i.checked_sub(1)?;
    }
}

/// Returns the index of the next non-newline, non-comment node after `idx`.
pub fn next_non_ws(nodes: &[Node<'_>], idx: usize) -> Option<usize> {
    let mut i = idx + 1;
    while i < nodes.len() {
        let tt = &nodes[i].token.token_type;
        if *tt != TokenType::Newline && *tt != TokenType::Comment {
            return Some(i);
        }
        i += 1;
    }
    None
}

// ── Table aliases in FROM clause ─────────────────────────────────────────────

#[derive(Debug)]
pub struct TableAlias {
    /// The alias string (lowercased for comparison).
    pub alias: String,
    /// Index of the alias token.
    pub alias_idx: usize,
    /// True if the AS keyword was used.
    #[allow(dead_code)]
    pub is_explicit: bool,
}

/// Find all table aliases defined in FROM/JOIN clauses using a pre-computed clause map.
/// Avoids recomputing `clause_map` when the caller already has one.
pub fn table_aliases_with_clauses<'a>(nodes: &[Node<'a>], clauses: &[Clause]) -> Vec<TableAlias> {
    let mut aliases: Vec<TableAlias> = Vec::new();
    let mut i = 0;

    while i < nodes.len() {
        if clauses[i] != Clause::From || nodes[i].bracket_depth != 0 {
            i += 1;
            continue;
        }

        let n = &nodes[i];

        // Pattern: table_name AS alias_name
        if n.token.token_type == TokenType::UntermintedKeyword && n.value.eq_ignore_ascii_case("as")
        {
            if let Some(alias_idx) = next_non_ws(nodes, i) {
                let at = &nodes[alias_idx];
                if at.token.token_type == TokenType::Name
                    || at.token.token_type == TokenType::QuotedName
                {
                    aliases.push(TableAlias {
                        alias: at.value.to_lowercase(),
                        alias_idx,
                        is_explicit: true,
                    });
                    i = alias_idx + 1;
                    continue;
                }
            }
        }

        // Pattern: table_name alias_name (no AS)
        if n.token.token_type == TokenType::Name || n.token.token_type == TokenType::QuotedName {
            if let Some(next_idx) = next_non_ws(nodes, i) {
                let next = &nodes[next_idx];
                if (next.token.token_type == TokenType::Name
                    || next.token.token_type == TokenType::QuotedName)
                    && clauses[next_idx] == Clause::From
                    && next.bracket_depth == 0
                    // Make sure it's not a keyword
                    && !is_clause_keyword(next.value)
                {
                    aliases.push(TableAlias {
                        alias: next.value.to_lowercase(),
                        alias_idx: next_idx,
                        is_explicit: false,
                    });
                    i = next_idx + 1;
                    continue;
                }
            }
        }

        i += 1;
    }

    aliases
}

/// Find all table aliases defined in FROM/JOIN clauses.
#[allow(dead_code)]
pub fn table_aliases(nodes: &[Node<'_>]) -> Vec<TableAlias> {
    table_aliases_with_clauses(nodes, &clause_map(nodes))
}

fn is_clause_keyword(value: &str) -> bool {
    matches_any_ci(
        value,
        &[
            "where",
            "group",
            "order",
            "having",
            "limit",
            "join",
            "inner",
            "left",
            "right",
            "full",
            "outer",
            "cross",
            "on",
            "using",
            "set",
            "union",
            "intersect",
            "except",
        ],
    )
}

// ── CTE detection ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct CteDef {
    pub name: String,
    pub name_idx: usize,
    /// Index of the `(` that opens the CTE body.
    #[allow(dead_code)]
    pub open_idx: usize,
    /// Index of the matching `)` that closes the CTE body.
    pub close_idx: usize,
}

/// Find all CTE definitions: `WITH name AS ( ... )`.
/// Populates `open_idx` (the `(`) and `close_idx` (the matching `)`) for each CTE.
pub fn cte_definitions(nodes: &[Node<'_>]) -> Vec<CteDef> {
    let mut defs: Vec<CteDef> = Vec::new();
    let mut i = 0;

    while i < nodes.len() {
        let n = &nodes[i];
        if n.token.token_type != TokenType::StatementStart || !n.value.eq_ignore_ascii_case("with")
        {
            i += 1;
            continue;
        }

        // Found WITH — scan for one or more `name AS ( ... )` definitions.
        let mut j = i + 1;
        loop {
            // Skip whitespace/comments.
            while j < nodes.len()
                && matches!(
                    nodes[j].token.token_type,
                    TokenType::Newline | TokenType::Comment
                )
            {
                j += 1;
            }
            if j >= nodes.len() {
                break;
            }

            // Comma between consecutive CTEs.
            if nodes[j].token.token_type == TokenType::Comma {
                j += 1;
                continue;
            }

            // Only handle top-level names (depth 0).
            if nodes[j].bracket_depth != 0 {
                break;
            }

            // Expect a CTE name.
            if !matches!(
                nodes[j].token.token_type,
                TokenType::Name | TokenType::QuotedName
            ) {
                break; // SELECT or other — end of CTE list.
            }

            let name_idx = j;
            let name = nodes[j].value.to_string();
            j += 1;

            // Skip whitespace to AS.
            while j < nodes.len()
                && matches!(
                    nodes[j].token.token_type,
                    TokenType::Newline | TokenType::Comment
                )
            {
                j += 1;
            }
            if j >= nodes.len() {
                break;
            }

            if nodes[j].token.token_type != TokenType::UntermintedKeyword
                || !nodes[j].value.eq_ignore_ascii_case("as")
            {
                break;
            }
            j += 1; // past AS

            // Skip whitespace to (.
            while j < nodes.len()
                && matches!(
                    nodes[j].token.token_type,
                    TokenType::Newline | TokenType::Comment
                )
            {
                j += 1;
            }
            if j >= nodes.len() {
                break;
            }

            if nodes[j].token.token_type != TokenType::BracketOpen || nodes[j].value != "(" {
                break;
            }
            let open_idx = j;
            let open_depth = nodes[open_idx].bracket_depth;

            // Find the matching `)`: bracket_depth == open_depth + 1.
            let close_rel = nodes[open_idx + 1..].iter().position(|n| {
                n.token.token_type == TokenType::BracketClose && n.bracket_depth == open_depth + 1
            });
            let Some(rel) = close_rel else { break };
            let close_idx = open_idx + 1 + rel;

            defs.push(CteDef {
                name,
                name_idx,
                open_idx,
                close_idx,
            });
            j = close_idx + 1; // advance past the CTE body to look for more CTEs
        }
        i = j;
    }

    defs
}

/// Check whether `name` appears as a reference anywhere in `nodes` (case-insensitive).
#[allow(dead_code)]
pub fn name_referenced(nodes: &[Node<'_>], name: &str) -> bool {
    nodes.iter().any(|n| {
        matches!(n.token.token_type, TokenType::Name | TokenType::QuotedName)
            && n.value.eq_ignore_ascii_case(name)
    })
}
