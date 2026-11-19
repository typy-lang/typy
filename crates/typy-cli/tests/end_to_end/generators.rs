//! End-to-end tests of generators freed while suspended (FUN-11): what closing one writes to
//! standard error holds the generator's address, which a conformance test cannot compare.

use crate::common::Program;

/// Returns `text` with each address `0x...` written as `0xADDRESS`.
fn without_addresses(text: &str) -> String {
    let mut written = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("0x") {
        written.push_str(&rest[..start]);
        written.push_str("0xADDRESS");
        rest = rest[start + 2..].trim_start_matches(|each: char| each.is_ascii_hexdigit());
    }
    written.push_str(rest);
    written
}

#[test]
fn fun_11_an_exception_a_freed_generator_raises_while_closing_is_written_and_ignored() {
    let program = Program::build(
        "from collections.abc import Iterator\n\n\ndef raising() -> Iterator[int]:\n    \
         try:\n        yield 1\n    finally:\n        raise ValueError(\"boom\")\n\n\n\
         def stubborn() -> Iterator[int]:\n    try:\n        yield 1\n    except \
         GeneratorExit:\n        yield 2\n\n\nnext(raising())\nnext(stubborn())\n\
         print(\"goes on\")\n",
    );
    let output = program.run(&[], b"");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "goes on\n");
    assert_eq!(
        without_addresses(&String::from_utf8_lossy(&output.stderr)),
        "Exception ignored in: <generator object raising at 0xADDRESS>\n\
         Traceback (most recent call last):\n  File \"app.py\", line 8, in raising\n\
         ValueError: boom\n\
         Exception ignored in: <generator object stubborn at 0xADDRESS>\n\
         RuntimeError: generator ignored GeneratorExit\n"
    );
}
