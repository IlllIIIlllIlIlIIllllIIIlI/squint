use std::fmt::{Display, Formatter, Result};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenType {
    FmtOff,
    FmtOn,
    Data,
    JinjaStatement,
    JinjaExpression,
    JinjaBlockStart,
    JinjaBlockEnd,
    JinjaBlockKeyword,
    QuotedName,
    Comment,
    Semicolon,
    StatementStart,
    Star,
    Number,
    BracketOpen,
    BracketClose,
    Colon,
    Operator,
    WordOperator,
    On,
    BooleanOperator,
    Comma,
    Dot,
    Newline,
    UntermintedKeyword,
    SetOperator,
    Name,
}

impl Display for TokenType {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:?}", self)
    }
}

/// Token metadata: type and source position only.
/// The token text lives in `Node::value`; the prefix lives in `Node::prefix`.
#[derive(Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub spos: usize,
    pub epos: usize,
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "type={}, spos={}, epos={}",
            self.token_type, self.spos, self.epos
        )
    }
}

impl TokenType {
    /// Returns true for tokens that carry no semantic content (newlines and comments).
    /// Use this when searching for the next meaningful token.
    pub fn is_whitespace_or_comment(self) -> bool {
        matches!(self, TokenType::Newline | TokenType::Comment)
    }

    /// Returns true for Jinja template tokens and fmt-off/on markers.
    /// These tokens have intentional surrounding whitespace that should not be linted.
    pub fn is_jinja(self) -> bool {
        matches!(
            self,
            TokenType::JinjaExpression
                | TokenType::JinjaStatement
                | TokenType::JinjaBlockStart
                | TokenType::JinjaBlockEnd
                | TokenType::JinjaBlockKeyword
                | TokenType::FmtOff
                | TokenType::FmtOn
        )
    }
}
