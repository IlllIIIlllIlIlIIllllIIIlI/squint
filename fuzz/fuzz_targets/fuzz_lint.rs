#![no_main]

use libfuzzer_sys::fuzz_target;
use squint::{build_rules, config::Config, linter::lint_source};

// Fuzz the full lint pipeline: lex → parse → all rules → violations.
// Goal: no panics on any valid UTF-8 input. Violations are expected and fine;
// a panic is a bug.
fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    let cfg = Config::default();
    let rules = build_rules(&cfg);
    let rule_refs: Vec<&dyn squint::rules::Rule> = rules.iter().map(|r| r.as_ref()).collect();
    let violations = lint_source(source, &rule_refs);

    // Sanity: every reported line/col must be reachable in the source.
    // (line 1, col 1 is always valid even for empty input.)
    for v in &violations {
        assert!(v.line >= 1);
        assert!(v.col >= 1);
    }
});
