use squint::linter::{fix_source, lint_source};
use squint::rules::{Rule, Violation};

pub fn check(rule: impl Rule + 'static, sql: &str) -> Vec<Violation> {
    let b: Box<dyn Rule> = Box::new(rule);
    lint_source(sql, &[b.as_ref()])
}

#[allow(dead_code)]
pub fn fix(rule: impl Rule + 'static, sql: &str) -> String {
    let b: Box<dyn Rule> = Box::new(rule);
    fix_source(sql, &[b.as_ref()])
}
