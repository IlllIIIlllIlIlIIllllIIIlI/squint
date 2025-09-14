use crate::tokens::{Token, TokenType};
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub struct Node<'a> {
    pub token: Token,
    previous_node: &'a Option<Node<'a>>,
    prefix: String,
    value: String,
    open_brackets: Vec<Node<'a>>,
    open_jinja_blocks: Vec<Node<'a>>,
    formatting_disabled: Vec<Token>,
}

impl Display for Node<'_> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}{}", self.prefix, self.value)
    }
}

impl Node<'_> {
    pub fn depth(&self) -> (usize, usize) {
        (self.open_brackets.len(), self.open_jinja_blocks.len())
    }

    pub fn is_unterminated_keyword(&self) -> bool {
        self.token.token_type == TokenType::UntermintedKeyword
    }

    pub fn is_comma(&self) -> bool {
        self.token.token_type == TokenType::Comma
    }

    pub fn is_bracket_operator(&self) -> bool {
        if self.token.token_type != TokenType::BracketOpen {
            return false;
        }

        let (prev, _) = get_previous_token(self.previous_node);
        if prev.is_none() {
            return false;
        }

        let unwrapped = prev.unwrap();
        if self.value == "[" {
            return [
                TokenType::Name,
                TokenType::QuotedName,
                TokenType::BracketClose,
            ]
            .contains(&unwrapped.token_type);
        }

        return self.value == "("
            && unwrapped.token_type == TokenType::BracketClose
            && unwrapped.token.contains(">");
    }

    pub fn is_multiplication_star(&self) -> bool {
        if self.token.token_type != TokenType::Star {
            return false;
        }

        let (prev, _) = get_previous_token(self.previous_node);
        if prev.is_none() {
            return false;
        }

        return ![
            TokenType::UntermintedKeyword,
            TokenType::Comma,
            TokenType::Dot,
        ]
        .contains(&prev.unwrap().token_type);
    }

    pub fn is_the_and_after_the_between_operator(&self) -> bool {
        if !self.is_boolean_operator() || self.value != "and" {
            return false;
        }

        return self.has_preceeding_between_operator();
    }

    fn is_boolean_operator(&self) -> bool {
        self.token.token_type == TokenType::BooleanOperator
    }

    fn is_between_operator(&self) -> bool {
        self.token.token_type == TokenType::WordOperator && self.value == "between"
    }

    fn has_preceeding_between_operator(&self) -> bool {
        let mut prev = match self.previous_node.is_some() {
            true => self.previous_node.as_ref().unwrap().previous_node,
            _ => &None,
        };

        while prev.is_some() && prev.as_ref().unwrap().depth() >= self.depth() {
            if prev.as_ref().unwrap().depth() == self.depth() {
                if prev.as_ref().unwrap().is_between_operator() {
                    return true;
                }
                if prev.as_ref().unwrap().is_boolean_operator() {
                    break;
                }
            }
            prev = prev.as_ref().unwrap().previous_node;
        }

        return false;
    }
}

pub fn get_previous_token<'a>(previous_node: &'a Option<Node<'a>>) -> (Option<&'a Token>, bool) {
    if previous_node.is_none() {
        return (None, false);
    }

    let unwrapped_node = previous_node.as_ref().unwrap();
    let t = &unwrapped_node.token;
    if t.token_type.does_not_set_prev_sql_context() {
        let (prev, _) = get_previous_token(unwrapped_node.previous_node);
        return (prev, true);
    }

    return (Some(t), false);
}
