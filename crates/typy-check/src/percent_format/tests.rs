//! Tests of `percent_format`.

use super::{Conversion, Number, Piece, parse};

fn conversion(
    position: usize,
    key: Option<&str>,
    flags: &str,
    width: Number,
    precision: Number,
    kind: char,
) -> Piece {
    Piece::Conversion(Conversion {
        position,
        key: key.map(str::to_owned),
        flags: flags.to_owned(),
        width,
        precision,
        kind,
    })
}

#[test]
fn a_format_is_literal_text_and_conversions() {
    assert_eq!(
        parse("a %-08.3f b %%%(x(y))r %*.*d %ld %5.s").unwrap(),
        [
            Piece::Literal("a ".to_owned()),
            conversion(2, None, "-0", Number::Given(8), Number::Given(3), 'f'),
            Piece::Literal(" b %".to_owned()),
            conversion(14, Some("x(y)"), "", Number::Absent, Number::Absent, 'r'),
            Piece::Literal(" ".to_owned()),
            conversion(23, None, "", Number::Star, Number::Star, 'd'),
            Piece::Literal(" ".to_owned()),
            conversion(29, None, "", Number::Absent, Number::Absent, 'd'),
            Piece::Literal(" ".to_owned()),
            conversion(33, None, "", Number::Given(5), Number::Given(0), 's'),
        ]
    );
    assert_eq!(parse("").unwrap(), []);
    assert_eq!(parse("%%").unwrap(), [Piece::Literal("%".to_owned())]);
}

#[test]
fn a_format_cpython_refuses_gives_its_message() {
    for (format, message) in [
        ("%", "stray % at position 0"),
        ("ab %-5", "stray % at position 3"),
        ("%(a)", "stray % at position 0"),
        ("%*", "stray % at position 0"),
        ("ab %(k", "stray % or incomplete format key at position 3"),
        ("%5y", "unsupported format %y at position 0"),
        ("%99999999999999999999d", "width too big"),
        ("%.3000000000d", "precision too big"),
        (
            "%(a)5!",
            "stray % at position 0 or unexpected format character '!' at position 5",
        ),
        (
            "%\n",
            "stray % at position 0 or unexpected format character U+000A at position 1",
        ),
        ("\u{e9}%y", "unsupported format %y at position 1"),
        (
            "%5%",
            "stray % at position 0 or unexpected format character '%' at position 2",
        ),
        (
            "%\u{e9}",
            "stray % at position 0 or unexpected format character '\u{e9}' (U+00E9) at position 1",
        ),
    ] {
        assert_eq!(parse(format), Err(message.to_owned()), "{format}");
    }
}
