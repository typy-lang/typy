//! Tests of `float(text)` against CPython's results and against Rust's correctly rounded parser.

use super::typy_rt_float_from_str;
use crate::string::{code_point, from_str, length};

fn parse(text: &str) -> Result<f64, String> {
    // SAFETY: the string is live, and so is any exception and its message.
    unsafe {
        let result = typy_rt_float_from_str(from_str(text));
        if result.exception.is_null() {
            return Ok(result.value);
        }
        let message = (*result.exception).message;
        Err((0..length(message))
            .map(|index| char::from_u32(code_point(message, index)).unwrap())
            .collect())
    }
}

#[test]
fn text_is_read_with_python_rules() {
    let cases: [(&str, f64); 20] = [
        ("\u{661}.\u{665}", 1.5),
        ("\u{661}e\u{662}", 100.0),
        (" 1_0.5 ", 10.5),
        ("+1E-3", 0.001),
        ("1e500", f64::INFINITY),
        (".5", 0.5),
        ("5.", 5.0),
        ("-inFinity", f64::NEG_INFINITY),
        ("INF", f64::INFINITY),
        ("-0", -0.0),
        ("1e-400", 0.0),
        ("0.1", 0.1),
        ("1e99999999999999999999", f64::INFINITY),
        ("1e-99999999999999999999", 0.0),
        ("\u{3000}12\n", 12.0),
        ("1_2e1_0", 12e10),
        ("2.2250738585072011e-308", 2.225_073_858_507_201e-308),
        ("179769313486231580793728971405301e276", f64::MAX),
        ("0.000000000000000000000000000000000000001", 1e-39),
        ("4.9406564584124654e-324", 5e-324),
    ];
    for (text, expected) in cases {
        let value = parse(text).unwrap_or_else(|message| panic!("{text:?}: {message}"));
        assert_eq!(value.to_bits(), expected.to_bits(), "{text:?}");
    }
    assert!(parse("nan").unwrap().is_nan());
    assert!(parse("-NaN").unwrap().is_nan());
}

#[test]
fn malformed_text_raises_with_the_quoted_input() {
    for text in [
        "",
        " ",
        "x",
        "1__0",
        "_1",
        "1_",
        "1_e5",
        "1e",
        "1e+",
        ".",
        "0x10",
        "1.5.2",
        "infinityx",
        "- 1",
        "1 2",
        "e5",
        "\u{661}x",
    ] {
        assert_eq!(
            parse(text),
            Err(format!("could not convert string to float: {text:?}").replace('"', "'")),
            "{text:?}"
        );
    }
}

#[test]
fn decimal_text_rounds_as_rust_reads_it() {
    let mut state = 0x853c_49e6_748f_ea9b_u64;
    for _ in 0..if cfg!(miri) { 20 } else { 5_000 } {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let digits = state % 100_000_000_000_000_000;
        let exponent = i64::try_from((state >> 40) % 700).unwrap() - 350;
        let text = format!("{digits}e{exponent}");
        assert_eq!(
            parse(&text).unwrap().to_bits(),
            text.parse::<f64>().unwrap().to_bits(),
            "{text}"
        );
    }
}
