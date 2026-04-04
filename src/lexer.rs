use crate::tokens::TokenType;
use logos::{Filter, Logos};

/// Compile-time perfect hash map: lowercase keyword → TokenType.
static KEYWORDS: phf::Map<&'static str, TokenType> = phf::phf_map! {
    // StatementStart
    "select"   => TokenType::StatementStart,
    "insert"   => TokenType::StatementStart,
    "update"   => TokenType::StatementStart,
    "delete"   => TokenType::StatementStart,
    "create"   => TokenType::StatementStart,
    "drop"     => TokenType::StatementStart,
    "alter"    => TokenType::StatementStart,
    "truncate" => TokenType::StatementStart,
    "merge"    => TokenType::StatementStart,
    "with"     => TokenType::StatementStart,
    // UntermintedKeyword
    "from"      => TokenType::UntermintedKeyword,
    "join"      => TokenType::UntermintedKeyword,
    "inner"     => TokenType::UntermintedKeyword,
    "left"      => TokenType::UntermintedKeyword,
    "right"     => TokenType::UntermintedKeyword,
    "full"      => TokenType::UntermintedKeyword,
    "outer"     => TokenType::UntermintedKeyword,
    "cross"     => TokenType::UntermintedKeyword,
    "where"     => TokenType::UntermintedKeyword,
    "group"     => TokenType::UntermintedKeyword,
    "order"     => TokenType::UntermintedKeyword,
    "having"    => TokenType::UntermintedKeyword,
    "limit"     => TokenType::UntermintedKeyword,
    "offset"    => TokenType::UntermintedKeyword,
    "set"       => TokenType::UntermintedKeyword,
    "into"      => TokenType::UntermintedKeyword,
    "values"    => TokenType::UntermintedKeyword,
    "returning" => TokenType::UntermintedKeyword,
    "partition" => TokenType::UntermintedKeyword,
    "over"      => TokenType::UntermintedKeyword,
    "window"    => TokenType::UntermintedKeyword,
    "case"      => TokenType::UntermintedKeyword,
    "when"      => TokenType::UntermintedKeyword,
    "then"      => TokenType::UntermintedKeyword,
    "else"      => TokenType::UntermintedKeyword,
    "end"       => TokenType::UntermintedKeyword,
    "as"        => TokenType::UntermintedKeyword,
    "distinct"  => TokenType::UntermintedKeyword,
    "all"       => TokenType::UntermintedKeyword,
    "top"       => TokenType::UntermintedKeyword,
    "by"        => TokenType::UntermintedKeyword,
    "asc"       => TokenType::UntermintedKeyword,
    "desc"      => TokenType::UntermintedKeyword,
    // WordOperator
    "between" => TokenType::WordOperator,
    "like"    => TokenType::WordOperator,
    "ilike"   => TokenType::WordOperator,
    "in"      => TokenType::WordOperator,
    "not"     => TokenType::WordOperator,
    "is"      => TokenType::WordOperator,
    "exists"  => TokenType::WordOperator,
    "any"     => TokenType::WordOperator,
    "some"    => TokenType::WordOperator,
    "similar" => TokenType::WordOperator,
    // BooleanOperator
    "and" => TokenType::BooleanOperator,
    "or"  => TokenType::BooleanOperator,
    // SetOperator
    "union"     => TokenType::SetOperator,
    "intersect" => TokenType::SetOperator,
    "except"    => TokenType::SetOperator,
    // On (its own type)
    "on" => TokenType::On,
};

// ── Jinja statement classification ───────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum JinjaKind {
    BlockStart,
    BlockKeyword,
    BlockEnd,
    Statement,
}

fn classify_jinja(content: &str) -> JinjaKind {
    let s = content.trim_matches('-').trim();
    let word = s.split_whitespace().next().unwrap_or("").to_lowercase();
    match word.as_str() {
        "if" | "for" | "macro" | "call" | "filter" | "set" | "block" | "raw" | "extends"
        | "include" | "import" | "from" => JinjaKind::BlockStart,
        "elif" | "else" => JinjaKind::BlockKeyword,
        w if w.starts_with("end") => JinjaKind::BlockEnd,
        _ => JinjaKind::Statement,
    }
}

// ── logos callbacks ───────────────────────────────────────────────────────────

fn lex_jinja_comment(lex: &mut logos::Lexer<RawTok>) -> Filter<()> {
    let rem = lex.remainder();
    match rem.find("#}") {
        Some(end) => lex.bump(end + 2),
        // Unterminated: consume to end so the opening `{#` is not silently
        // dropped from pending_prefix (same class of bug as lex_string).
        None => lex.bump(rem.len()),
    }
    Filter::Emit(())
}

fn lex_jinja_expr(lex: &mut logos::Lexer<RawTok>) -> Filter<()> {
    let rem = lex.remainder();
    match rem.find("}}") {
        Some(end) => lex.bump(end + 2),
        None => lex.bump(rem.len()),
    }
    Filter::Emit(())
}

fn lex_jinja_stmt(lex: &mut logos::Lexer<RawTok>) -> Filter<JinjaKind> {
    let rem = lex.remainder();
    match rem.find("%}") {
        Some(end) => {
            let kind = classify_jinja(&rem[..end]);
            lex.bump(end + 2);
            Filter::Emit(kind)
        }
        None => {
            lex.bump(rem.len());
            Filter::Emit(JinjaKind::Statement)
        }
    }
}

fn lex_block_comment(lex: &mut logos::Lexer<RawTok>) -> Filter<()> {
    let rem = lex.remainder();
    match rem.find("*/") {
        Some(end) => lex.bump(end + 2),
        None => lex.bump(rem.len()),
    }
    Filter::Emit(())
}

fn lex_string(lex: &mut logos::Lexer<RawTok>) -> Filter<()> {
    let bytes = lex.remainder().as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' => {
                lex.bump(i + 1);
                return Filter::Emit(());
            }
            b'\\' => i += 2,
            _ => i += 1,
        }
    }
    // Unterminated string literal: consume remaining bytes and emit.
    // Returning Filter::Skip would silently drop the opening `'` from
    // pending_prefix, causing the next token's prefix to have a shorter
    // byte length than the actual gap — which can put fix offsets inside
    // a multi-byte character and panic in replace_range.
    lex.bump(bytes.len());
    Filter::Emit(())
}

// ── DFA token enum ────────────────────────────────────────────────────────────

#[derive(Logos, Debug, PartialEq)]
enum RawTok {
    #[regex(r"--[^\n]*", allow_greedy = true)]
    LineComment,

    #[regex(r"#[^\n]*", allow_greedy = true)]
    HashComment,

    #[token("{#", lex_jinja_comment)]
    JinjaComment,

    #[token("{{", lex_jinja_expr)]
    JinjaExpr,

    #[token("{%", lex_jinja_stmt)]
    JinjaStmt(JinjaKind),

    #[token("/*", lex_block_comment)]
    BlockComment,

    #[regex(r"`[^`]*`")]
    #[regex(r#""[^"]*""#)]
    #[regex(r"\[[^\]]*\]")]
    QuotedName,

    #[token("'", lex_string)]
    StringLit,

    #[regex(r"[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?")]
    Number,

    #[token(";")]
    Semi,

    #[token("(")]
    #[token("[", priority = 2)]
    LParen,

    #[token(")")]
    #[token("]", priority = 2)]
    RParen,

    #[token("*")]
    Star,

    #[token(",")]
    Comma,

    #[token(".")]
    Dot,

    #[token("::")]
    DblColon,

    #[token(":")]
    Colon,

    #[regex(r"!=|<>|<=|>=|->|->>|\|\|")]
    MultiOp,

    #[regex(r"[<>=]")]
    CmpOp,

    #[regex(r"[+\-/%^&|~]")]
    ArithOp,

    #[regex(r"[ \t\r]+")]
    Space,

    #[token("\n")]
    Newline,

    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    Name,
}

fn raw_to_type(raw: &RawTok) -> TokenType {
    match raw {
        RawTok::LineComment | RawTok::HashComment | RawTok::JinjaComment | RawTok::BlockComment => {
            TokenType::Comment
        }
        RawTok::JinjaExpr => TokenType::JinjaExpression,
        RawTok::JinjaStmt(JinjaKind::BlockStart) => TokenType::JinjaBlockStart,
        RawTok::JinjaStmt(JinjaKind::BlockKeyword) => TokenType::JinjaBlockKeyword,
        RawTok::JinjaStmt(JinjaKind::BlockEnd) => TokenType::JinjaBlockEnd,
        RawTok::JinjaStmt(JinjaKind::Statement) => TokenType::JinjaStatement,
        RawTok::QuotedName => TokenType::QuotedName,
        RawTok::StringLit => TokenType::Data,
        RawTok::Number => TokenType::Number,
        RawTok::Semi => TokenType::Semicolon,
        RawTok::LParen => TokenType::BracketOpen,
        RawTok::RParen => TokenType::BracketClose,
        RawTok::Star => TokenType::Star,
        RawTok::Comma => TokenType::Comma,
        RawTok::Dot => TokenType::Dot,
        RawTok::DblColon | RawTok::Colon => TokenType::Colon,
        RawTok::MultiOp | RawTok::CmpOp | RawTok::ArithOp => TokenType::Operator,
        RawTok::Name => TokenType::Name,
        RawTok::Space | RawTok::Newline => unreachable!(),
    }
}

// ── Intermediate token (lexer output, parser input) ───────────────────────────

/// Output of the lexer, input to the parser.
/// `value` is a zero-copy slice into the source string.
pub struct LexedToken<'src> {
    pub token_type: TokenType,
    pub prefix: String,
    pub value: &'src str,
    pub spos: usize,
    pub epos: usize,
}

// ── Public Lexer ──────────────────────────────────────────────────────────────

pub struct Lexer<'src> {
    source: &'src str,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Lexer { source }
    }

    pub fn tokenize(&mut self) -> Vec<LexedToken<'src>> {
        let mut lex = RawTok::lexer(self.source);
        let mut tokens = Vec::new();
        let mut pending_prefix = String::new();

        while let Some(result) = lex.next() {
            let span = lex.span();

            match result {
                Err(_) => {
                    pending_prefix.push_str(lex.slice());
                }
                Ok(RawTok::Space) => {
                    pending_prefix.push_str(lex.slice());
                }
                Ok(RawTok::Newline) => {
                    tokens.push(LexedToken {
                        token_type: TokenType::Newline,
                        prefix: std::mem::take(&mut pending_prefix),
                        value: lex.slice(),
                        spos: span.start,
                        epos: span.end,
                    });
                }
                Ok(RawTok::LineComment) => {
                    // Only line comments (`--`) can be fmt:off/fmt:on markers.
                    let value = lex.slice();
                    tokens.push(LexedToken {
                        token_type: classify_comment(value),
                        prefix: std::mem::take(&mut pending_prefix),
                        value,
                        spos: span.start,
                        epos: span.end,
                    });
                }
                Ok(raw) => {
                    let value = lex.slice();
                    let tt = raw_to_type(&raw);
                    let token_type = if tt == TokenType::Name {
                        refine_keyword(value)
                    } else {
                        tt
                    };
                    tokens.push(LexedToken {
                        token_type,
                        prefix: std::mem::take(&mut pending_prefix),
                        value,
                        spos: span.start,
                        epos: span.end,
                    });
                }
            }
        }

        tokens
    }
}

// ── Comment classification ────────────────────────────────────────────────────

/// Reclassify a line comment as `FmtOff` or `FmtOn` if it matches the marker
/// syntax `-- fmt: off` / `-- fmt: on` (whitespace around `:` is flexible).
/// All other line comments remain `Comment`.
fn classify_comment(s: &str) -> TokenType {
    // Fast path: the vast majority of comments don't contain "fmt:".
    if !s.contains("fmt:") {
        return TokenType::Comment;
    }
    // Strip leading dashes and optional whitespace to reach the comment body.
    let body = s.trim_start_matches('-').trim_start_matches([' ', '\t']);
    if let Some(rest) = body.strip_prefix("fmt:") {
        let rest = rest.trim_start_matches([' ', '\t']);
        if rest.starts_with("off") {
            return TokenType::FmtOff;
        }
        if rest.starts_with("on") {
            return TokenType::FmtOn;
        }
    }
    TokenType::Comment
}

// ── Keyword refinement ────────────────────────────────────────────────────────

/// Look up a Name token in the keyword map (O(1) PHF).
/// Returns the keyword TokenType, or Name if not a keyword.
fn refine_keyword(text: &str) -> TokenType {
    if let Some(&kw_type) = KEYWORDS.get(text) {
        return kw_type;
    }
    let lower = text.to_lowercase();
    if let Some(&kw_type) = KEYWORDS.get(lower.as_str()) {
        return kw_type;
    }
    TokenType::Name
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn token_types(sql: &str) -> Vec<TokenType> {
        Lexer::new(sql)
            .tokenize()
            .into_iter()
            .map(|t| t.token_type)
            .collect()
    }

    #[test]
    fn test_basic_select() {
        let types = token_types("SELECT id FROM t");
        assert_eq!(
            types,
            vec![
                TokenType::StatementStart,
                TokenType::Name,
                TokenType::UntermintedKeyword,
                TokenType::Name,
            ]
        );
    }

    #[test]
    fn test_lowercase_keywords_classified() {
        let types = token_types("select id from t");
        assert_eq!(types[0], TokenType::StatementStart);
        assert_eq!(types[2], TokenType::UntermintedKeyword);
    }

    #[test]
    fn test_boolean_operators() {
        let types = token_types("a AND b OR c");
        assert_eq!(types[1], TokenType::BooleanOperator);
        assert_eq!(types[3], TokenType::BooleanOperator);
    }

    #[test]
    fn test_comment_dash() {
        let types = token_types("SELECT -- a comment\nid");
        assert!(types.contains(&TokenType::Comment));
        assert!(types.contains(&TokenType::Newline));
    }

    #[test]
    fn test_jinja_expression() {
        let types = token_types("SELECT {{ my_col }} FROM t");
        assert!(types.contains(&TokenType::JinjaExpression));
    }

    #[test]
    fn test_quoted_name() {
        let types = token_types(r#"SELECT "my_col" FROM t"#);
        assert!(types.contains(&TokenType::QuotedName));
    }

    #[test]
    fn test_string_literal() {
        let types = token_types("WHERE status = 'active'");
        assert!(types.contains(&TokenType::Data));
    }

    #[test]
    fn test_number() {
        let types = token_types("WHERE x > 42");
        assert!(types.contains(&TokenType::Number));
    }

    #[test]
    fn test_prefix_captured() {
        let tokens = Lexer::new("SELECT  id").tokenize();
        assert_eq!(tokens[1].prefix, "  ");
    }

    #[test]
    fn test_jinja_block() {
        let types = token_types("{% if condition %}SELECT 1{% endif %}");
        assert!(types.contains(&TokenType::JinjaBlockStart));
        assert!(types.contains(&TokenType::JinjaBlockEnd));
    }

    #[test]
    fn test_operators() {
        let types = token_types("a != b AND c >= d");
        assert!(types.contains(&TokenType::Operator));
        assert!(types.contains(&TokenType::BooleanOperator));
    }

    #[test]
    fn test_unterminated_string_does_not_drop_bytes() {
        // Regression: "s<multibyte>'m" — the unterminated string literal `'m`
        // previously returned Filter::Skip, silently dropping `'` from pending_prefix.
        // This caused the next token's prefix byte length to be shorter than the actual
        // byte gap, producing Fix offsets inside a multi-byte character → panic in
        // replace_range.  Now unterminated strings are emitted as a StringLit token.
        let source = "s\u{07D5}'m"; // bytes: s DF 95 ' m
        let tokens = Lexer::new(source).tokenize();
        // The `'m` fragment must appear as a token (StringLit/Data), not be silently dropped.
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Data));
    }
}
