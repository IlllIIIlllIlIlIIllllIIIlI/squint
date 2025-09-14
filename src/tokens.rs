use regex::Captures;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug, PartialEq)]
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
    CommentStart,
    CommentEnd,
    Semicolon,
    StatementStart,
    StatementEnd,
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

impl TokenType {
    fn is_jinja_statement(&self) -> bool {
        [
            TokenType::JinjaStatement,
            TokenType::JinjaBlockStart,
            TokenType::JinjaBlockKeyword,
            TokenType::JinjaBlockEnd,
        ]
        .contains(self)
    }

    pub fn does_not_set_prev_sql_context(&self) -> bool {
        self.is_jinja_statement() || *self == TokenType::Newline
    }
}

#[derive(Debug)]
pub struct Token {
    pub token_type: TokenType,
    prefix: String,
    pub token: String,
    spos: usize,
    epos: usize,
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "type={}, token={}, spos={}, epos={}",
            self.token_type, self.token, self.spos, self.epos
        )
    }
}

impl Token {
    pub fn from_match(
        &self,
        source_string: &String,
        caps: &Captures,
        token_type: TokenType,
    ) -> Token {
        let whole_match = caps.get(0).unwrap();
        let sub_match = caps.get(1).unwrap();

        let pos = whole_match.start();
        let spos = sub_match.start();
        let epos = sub_match.end();

        let prefix = &source_string[pos..spos];
        let token_text = &source_string[spos..epos];

        Token {
            token_type,
            prefix: prefix.to_string(),
            token: token_text.to_string(),
            spos: pos,
            epos: epos,
        }
    }
}
