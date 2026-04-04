pub mod analysis;
pub mod config;
pub mod lexer;
pub mod linter;
pub mod node;
pub mod parser;
pub mod rules;
pub mod tokens;

use config::Config;
use rules::{
    aliasing::{
        AliasLength, ExpressionAliases, ImplicitAliases, SelfAliases, UniqueColumnAliases,
        UniqueTableAliases, UnusedAliases,
    },
    ambiguous::{ColumnReferences, DistinctGroupBy, ImplicitJoins, UnionDistinct},
    capitalisation::{FunctionCasing, IdentifierCasing, KeywordCasing, LiteralCasing, TypeCasing},
    convention::{CountRows, IsNull, QuotingStyle, TrailingComma},
    jinja::JinjaPadding,
    layout::{
        ClauseOrdering, CteBracket, CteNewline, EndOfFile, FunctionSpacing, Indent, LineLength,
        SelectModifiers, SetOperators, Spacing, TrailingWhitespace,
    },
    references::{References, WildcardColumns},
    structure::{Distinct, UnusedCte},
    Rule, Severity,
};

/// Build the full set of rules from config, applying severity overrides.
pub fn build_rules(cfg: &Config) -> Vec<Box<dyn Rule>> {
    let raw: Vec<Box<dyn Rule>> = vec![
        Box::new(KeywordCasing),
        Box::new(IdentifierCasing),
        Box::new(FunctionCasing),
        Box::new(LiteralCasing),
        Box::new(TypeCasing),
        Box::new(ImplicitAliases),
        Box::new(ExpressionAliases),
        Box::new(UniqueTableAliases),
        Box::new(UnusedAliases),
        Box::new(AliasLength::new(
            cfg.rules.al06.min_alias_length,
            cfg.rules.al06.max_alias_length,
        )),
        Box::new(UniqueColumnAliases),
        Box::new(SelfAliases),
        Box::new(DistinctGroupBy),
        Box::new(UnionDistinct),
        Box::new(ImplicitJoins),
        Box::new(ColumnReferences::new(
            cfg.rules.am06.group_by_and_order_by_style.clone(),
        )),
        Box::new(TrailingComma::new(
            cfg.rules.cv03.select_clause_trailing_comma.clone(),
        )),
        Box::new(CountRows::new(cfg.rules.cv04.prefer_count_1)),
        Box::new(IsNull),
        Box::new(QuotingStyle),
        Box::new(JinjaPadding),
        Box::new(Spacing),
        Box::new(Indent),
        Box::new(TrailingWhitespace),
        Box::new(LineLength::new(cfg.rules.lt05.max_line_length)),
        Box::new(FunctionSpacing),
        Box::new(CteBracket),
        Box::new(CteNewline),
        Box::new(ClauseOrdering),
        Box::new(SelectModifiers),
        Box::new(SetOperators),
        Box::new(EndOfFile),
        Box::new(References),
        Box::new(WildcardColumns),
        Box::new(UnusedCte),
        Box::new(Distinct),
    ];

    if cfg.rules.severity.is_empty() {
        return raw;
    }
    raw.into_iter()
        .map(|rule| {
            let id = rule.id().to_uppercase();
            if let Some(sev_str) = cfg.rules.severity.get(&id) {
                let sev = match sev_str.to_lowercase().as_str() {
                    "warning" => Severity::Warning,
                    _ => Severity::Error,
                };
                Box::new(BoxedWithSeverity {
                    inner: rule,
                    severity: sev,
                }) as Box<dyn Rule>
            } else {
                rule
            }
        })
        .collect()
}

/// Wraps a boxed `Rule` to override its severity. Used when config sets per-rule severity.
pub struct BoxedWithSeverity {
    pub inner: Box<dyn Rule>,
    pub severity: Severity,
}

impl Rule for BoxedWithSeverity {
    fn id(&self) -> &'static str {
        self.inner.id()
    }
    fn check(&self, ctx: &rules::LintContext<'_>) -> Vec<rules::Violation> {
        self.inner.check(ctx)
    }
    fn fixes(&self, nodes: &[node::Node<'_>], source: &str) -> Vec<rules::Fix> {
        self.inner.fixes(nodes, source)
    }
    fn severity(&self) -> Severity {
        self.severity
    }
}
