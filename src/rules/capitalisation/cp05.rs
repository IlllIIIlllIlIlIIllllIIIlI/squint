use crate::node::Node;
use crate::rules::{Fix, LintContext, Rule, Severity, Violation};
use crate::tokens::TokenType;

const DATA_TYPES: &[&str] = &[
    "int",
    "integer",
    "bigint",
    "smallint",
    "tinyint",
    "byteint",
    "float",
    "float4",
    "float8",
    "double",
    "real",
    "decimal",
    "numeric",
    "number",
    "money",
    "varchar",
    "nvarchar",
    "char",
    "nchar",
    "character",
    "string",
    "text",
    "boolean",
    "bool",
    "date",
    "time",
    "timestamp",
    "datetime",
    "timestamptz",
    "timetz",
    "interval",
    "blob",
    "binary",
    "varbinary",
    "bytes",
    "json",
    "jsonb",
    "xml",
    "uuid",
    "inet",
    "cidr",
    "macaddr",
    "array",
    "struct",
    "variant",
    "object",
];

/// CP05 — Data type names must be lowercase.
pub struct TypeCasing;

impl Rule for TypeCasing {
    fn id(&self) -> &'static str {
        "CP05"
    }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let (nodes, _source, line_index) = (ctx.nodes, ctx.source, ctx.line_index);
        nodes
            .iter()
            .filter(|n| {
                n.token.token_type == TokenType::Name
                    && DATA_TYPES.iter().any(|t| n.value.eq_ignore_ascii_case(t))
                    && n.value.bytes().any(|b| b.is_ascii_uppercase())
            })
            .map(|n| {
                let (line, col) = line_index.offset_to_line_col(n.token.spos);
                Violation {
                    line,
                    col,
                    rule_id: self.id(),
                    message: format!(
                        "Data types must be lowercase: found '{}', expected '{}'",
                        n.value,
                        n.value.to_lowercase()
                    ),
                    severity: Severity::Error,
                }
            })
            .collect()
    }

    fn fixes(&self, nodes: &[Node<'_>], _source: &str) -> Vec<Fix> {
        nodes
            .iter()
            .filter(|n| {
                n.token.token_type == TokenType::Name
                    && DATA_TYPES.iter().any(|t| n.value.eq_ignore_ascii_case(t))
                    && n.value.bytes().any(|b| b.is_ascii_uppercase())
            })
            .map(|n| Fix {
                start: n.token.spos,
                end: n.token.epos,
                replacement: n.value.to_lowercase(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(TypeCasing, sql)
    }

    #[test]
    fn test_clean() {
        assert!(violations("cast(x as varchar)").is_empty());
    }

    #[test]
    fn test_uppercase_type() {
        let v = violations("cast(x as VARCHAR)");
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_mixed_types() {
        let v = violations("cast(x as INT) + cast(y as BIGINT)");
        assert_eq!(v.len(), 2);
    }
}
