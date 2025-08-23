use crate::{node::Node, tokens::Token};
use regex::Regex;
use std::fmt::{Display, Formatter, Result};
use std::sync::OnceLock;

#[derive(Debug)]
pub struct Comment<'a> {
    token: Token,
    is_standalone: bool,
    previous_node: Option<Node<'a>>,
    _comment_marker: OnceLock<Regex>,
}

// impl Display for Comment<'_> {
//     fn fmt(&self, f: &mut Formatter) -> Result {
//         write!(f, "{}{}", self.prefix, self.value)
//     }
// }

impl Comment<'_> {
    fn new<'a>(token: Token, is_standalone: bool, previous_node: Option<Node<'a>>) -> Comment<'a> {
        Comment {
            token,
            is_standalone,
            previous_node,
            _comment_marker: OnceLock::new(),
        }
    }

    fn comment_marker(&self) -> Regex {
        self._comment_marker
            .get_or_init(|| Regex::new(r"(--|#|//|/\*|\{#-?)([^\S\n]*)").unwrap())
            .clone()
    }

    fn get_marker(&self) -> (String, usize) {
        let caps = self.comment_marker().captures(&self.token.token);
        if caps.is_none() {
            panic!("{} does not match comment marker", self.token.token)
        }

        let unwrapped = caps.unwrap();

        let first = unwrapped.get(1).unwrap();
        let second = unwrapped.get(2).unwrap();

        let epos = first.end();
        let text_offset = second.end();

        return (self.token.token[..epos].to_string(), text_offset);
    }

    fn rewrite_marker(&self, marker: String) -> String {
        if marker == "//" {
            return "--".to_string();
        }

        return marker;
    }

    fn comment_parts(&self) -> (String, String) {
        if self.is_multiline() {
            panic!()
        }
        let (marker, skips) = self.get_marker();
        let comment_text = &self.token.token[skips..];
        return (self.rewrite_marker(marker), comment_text.to_string());
    }

    fn is_multiline(&self) -> bool {
        self.token.token.contains("\n")
    }
}
