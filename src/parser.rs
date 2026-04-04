use crate::lexer::LexedToken;
use crate::node::Node;
use crate::tokens::{Token, TokenType};

pub struct Parser<'src> {
    tokens: Vec<LexedToken<'src>>,
}

impl<'src> Parser<'src> {
    pub fn new(tokens: Vec<LexedToken<'src>>) -> Self {
        Parser { tokens }
    }

    pub fn parse(self) -> Vec<Node<'src>> {
        let mut nodes: Vec<Node<'src>> = Vec::with_capacity(self.tokens.len());
        let mut bracket_stack: Vec<usize> = Vec::new();
        let mut jinja_stack: Vec<usize> = Vec::new();

        for lt in self.tokens {
            let token_type = lt.token_type;
            let node = Node {
                token: Token {
                    token_type,
                    spos: lt.spos,
                    epos: lt.epos,
                },
                prefix: lt.prefix, // moved, no clone
                value: lt.value,   // zero-copy &'src str
                bracket_depth: bracket_stack.len(),
            };

            let idx = nodes.len();
            nodes.push(node);

            match token_type {
                TokenType::BracketOpen => bracket_stack.push(idx),
                TokenType::BracketClose => {
                    bracket_stack.pop();
                }
                TokenType::JinjaBlockStart => jinja_stack.push(idx),
                TokenType::JinjaBlockEnd => {
                    jinja_stack.pop();
                }
                _ => {}
            }
        }

        nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(sql: &str) -> Vec<Node<'_>> {
        let tokens = Lexer::new(sql).tokenize();
        Parser::new(tokens).parse()
    }

    #[test]
    fn test_basic_parse() {
        let nodes = parse("SELECT id FROM t");
        assert_eq!(nodes.len(), 4);
        assert_eq!(nodes[0].value, "SELECT");
        assert_eq!(nodes[2].value, "FROM");
    }

    #[test]
    fn test_bracket_depth() {
        let nodes = parse("SELECT count(id) FROM t");
        let count_node = nodes.iter().find(|n| n.value == "count").unwrap();
        let id_node = nodes.iter().find(|n| n.value == "id").unwrap();
        assert_eq!(count_node.bracket_depth, 0);
        assert_eq!(id_node.bracket_depth, 1);
    }
}
