use crate::tokens::Token;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub struct Node<'src> {
    pub token: Token,
    pub prefix: String,
    /// Zero-copy slice into the source string.
    pub value: &'src str,
    /// Bracket nesting depth at this node (number of open brackets).
    pub bracket_depth: usize,
}

impl Display for Node<'_> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}{}", self.prefix, self.value)
    }
}
