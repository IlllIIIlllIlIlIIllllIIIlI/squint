#![no_main]

use libfuzzer_sys::fuzz_target;
use squint::lexer::Lexer;

// Fuzz the lexer in isolation.
// Goal: no panics, no infinite loops, no OOM on arbitrary UTF-8 input.
// The lexer must always terminate and return a well-formed token list.
fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    // Basic sanity: every token's byte span must be within the source.
    for tok in &tokens {
        assert!(tok.spos <= tok.epos);
        assert!(tok.epos <= source.len());
    }
});
