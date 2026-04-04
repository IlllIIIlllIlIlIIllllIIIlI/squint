#![no_main]

use libfuzzer_sys::fuzz_target;
use squint::{build_rules, config::Config, linter::fix_source};

// Fuzz the fix pipeline: repeatedly apply fixes until stable (or max passes).
// Goal: no panics; the fixer must always converge (never loop infinitely).
// fix_source internally caps at 10 passes so termination is guaranteed by
// design — this target verifies there are no panics along the way.
fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    let cfg = Config::default();
    let rules = build_rules(&cfg);
    let rule_refs: Vec<&dyn squint::rules::Rule> = rules.iter().map(|r| r.as_ref()).collect();
    let fixed = fix_source(source, &rule_refs);

    // The fixed output must be valid UTF-8 (it is a String, so always true,
    // but this assertion documents the invariant explicitly).
    assert!(std::str::from_utf8(fixed.as_bytes()).is_ok());
});
