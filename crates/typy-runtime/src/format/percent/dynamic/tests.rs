//! Tests of `%` formatting with a format known only when the program runs.

use super::typy_rt_percent_dynamic;
use crate::list::ItemKind;
use crate::refcount::typy_rt_decrement;
use crate::string::{StrObject, code_point, from_str, length};
use crate::tuple::{Field, TupleType};
use crate::word::inline;

fn text(string: *const StrObject) -> String {
    // SAFETY: the string is live.
    unsafe {
        (0..length(string))
            .map(|index| char::from_u32(code_point(string, index)).unwrap())
            .collect()
    }
}

/// Returns `format % values` of `int`s, or the message it raises.
fn format_ints(format: &str, values: &[i64], one: bool) -> Result<String, String> {
    let items: Vec<(ItemKind, u64)> = values
        .iter()
        .map(|value| (ItemKind::Int, inline(*value).unwrap().cast_unsigned()))
        .collect();
    format_items(format, &items, i64::from(one))
}

/// Returns `format % values` of items of these kinds given in this form, or the message
/// it raises.
fn format_items(format: &str, items: &[(ItemKind, u64)], form: i64) -> Result<String, String> {
    let fields: Vec<Field> = items
        .iter()
        .enumerate()
        .map(|(index, (kind, _))| Field::plain(*kind, u32::try_from(index * 8).unwrap()))
        .collect();
    let fields = fields.leak();
    let layout = Box::leak(Box::new(TupleType {
        size: 8 * items.len().max(1) as u64,
        length: items.len() as u64,
        fields: fields.as_ptr(),
    }));
    let words: Vec<u64> = items.iter().map(|(_, word)| *word).collect();
    let format = from_str(format);
    let form = inline(form).unwrap();
    // SAFETY: the format is live, and the words are a live tuple of the layout.
    unsafe {
        let found = typy_rt_percent_dynamic(
            format,
            layout,
            words.as_ptr().expose_provenance() as u64,
            form,
        );
        typy_rt_decrement(format.cast());
        if found.exception.is_null() {
            let written = text(found.value);
            typy_rt_decrement(found.value.cast());
            Ok(written)
        } else {
            let str = (*found.exception).class().str.unwrap();
            let message = str(found.exception.cast());
            let written = text(message.value);
            typy_rt_decrement(message.value.cast());
            typy_rt_decrement(found.exception.cast());
            Err(written)
        }
    }
}

#[test]
#[cfg_attr(
    miri,
    ignore = "building a 4300-digit integer takes hours under Miri; the same code runs in the other tests"
)]
fn an_int_of_more_than_4300_digits_is_not_written() {
    let mut big = crate::int::value::Integer::from_i128(1);
    let ten = crate::int::value::Integer::from_i128(10);
    for _ in 0..4300 {
        big = big.multiply(&ten);
    }
    let word = big.into_word();
    for format in ["%s", "%r", "%a"] {
        let found = format_items(format, &[(ItemKind::Int, word.cast_unsigned())], 1);
        let message = found.unwrap_err();
        assert!(
            message.starts_with("Exceeds the limit (4300 digits) for integer string conversion"),
            "{format}: {message}"
        );
    }
    // SAFETY: the test owns the word.
    unsafe { crate::refcount::typy_rt_int_decrement(word) };
}

#[test]
fn a_format_is_read_with_its_flags_widths_and_stars() {
    let cases: &[(&str, &[i64], bool, &str)] = &[
        (
            "%d-%05d|%-4x|%#o",
            &[1, 42, 255, 8],
            false,
            "1-00042|ff  |0o10",
        ),
        ("%*d|%-*d|", &[5, 42, -4, 7], false, "   42|7   |"),
        ("%s%%", &[50], true, "50%"),
        ("%c%c", &[72, 105], false, "Hi"),
        ("%.3f %e", &[2, 3], false, "2.000 3.000000e+00"),
        ("%ld|%hi|%Lu", &[5, 6, 7], false, "5|6|7"),
        ("plain", &[], false, "plain"),
    ];
    for (format, values, one, expected) in cases {
        assert_eq!(
            format_ints(format, values, *one).as_deref(),
            Ok(*expected),
            "{format}"
        );
    }
}

#[test]
fn a_format_that_does_not_fit_gives_cpythons_message() {
    let cases: &[(&str, &[i64], bool, &str)] = &[
        (
            "%d %d",
            &[1],
            false,
            "not enough arguments for format string (got 1)",
        ),
        (
            "%d",
            &[1, 2],
            false,
            "not all arguments converted during string formatting (required 1, got 2)",
        ),
        ("%", &[1], true, "stray % at position 0"),
        ("ab %y", &[1], true, "unsupported format %y at position 3"),
        (
            "%5!",
            &[1],
            true,
            "stray % at position 0 or unexpected format character '!' at position 2",
        ),
        (
            "%(k",
            &[1],
            true,
            "stray % or incomplete format key at position 0",
        ),
        ("%(k)s", &[1], true, "format requires a mapping, not int"),
        (
            "%(k)s",
            &[1, 2],
            false,
            "format requires a mapping, not tuple",
        ),
        (
            "%c",
            &[0x11_0000],
            true,
            "format argument: %c argument not in range(0x110000)",
        ),
    ];
    for (format, values, one, expected) in cases {
        assert_eq!(
            format_ints(format, values, *one),
            Err((*expected).to_owned()),
            "{format}"
        );
    }
}

fn text_word(text: &str) -> u64 {
    from_str(text).expose_provenance() as u64
}

/// A format, its values, and what CPython gives: the text, or the message it raises.
type Case = (
    &'static str,
    Vec<(ItemKind, u64)>,
    Result<&'static str, &'static str>,
);

fn float(value: f64) -> (ItemKind, u64) {
    (ItemKind::Float, value.to_bits())
}

#[test]
fn each_kind_of_value_is_converted_as_cpython_converts_it() {
    for (format, items, expected) in converted_cases() {
        let found = format_items(format, &items, 0);
        match expected {
            Ok(text) => assert_eq!(found.as_deref(), Ok(text), "{format}"),
            Err(message) => assert_eq!(found, Err(message.to_owned()), "{format}"),
        }
    }
}

fn converted_cases() -> Vec<Case> {
    vec![
        (
            "%s|%r|%a",
            vec![(ItemKind::Object, text_word("\u{e9}")); 3],
            Ok("\u{e9}|'\u{e9}'|'\\xe9'"),
        ),
        (
            "%d %i %u",
            vec![float(2.9), (ItemKind::Bool, 1), float(-1.5)],
            Ok("2 1 -1"),
        ),
        (
            "%x %s %s",
            vec![
                (ItemKind::Bool, 1),
                (ItemKind::NoneValue, 0),
                (ItemKind::Bool, 0),
            ],
            Ok("1 None False"),
        ),
        (
            "%.1f %g %E",
            vec![
                (ItemKind::Bool, 1),
                float(0.5),
                (ItemKind::Int, inline(3).unwrap().cast_unsigned()),
            ],
            Ok("1.0 0.5 3.000000E+00"),
        ),
        (
            "%c|%-3c|",
            vec![(ItemKind::Object, text_word("z")), (ItemKind::Bool, 1)],
            Ok("z|\u{1}  |"),
        ),
        (
            "%*s|%.*s",
            vec![
                (ItemKind::Bool, 1),
                (ItemKind::Object, text_word("ab")),
                (ItemKind::Bool, 0),
                (ItemKind::Object, text_word("cd")),
            ],
            Ok("ab|"),
        ),
        (
            "%d",
            vec![float(f64::NAN)],
            Err("cannot convert float NaN to integer"),
        ),
        (
            "%x",
            vec![float(1.5)],
            Err("format argument 1: %x requires an integer, not float"),
        ),
        (
            "%f",
            vec![(ItemKind::Object, text_word("s"))],
            Err("format argument 1: %f requires a real number, not str"),
        ),
        (
            "%c",
            vec![(ItemKind::Object, text_word("long"))],
            Err(
                "format argument 1: %c requires an integer or a unicode character, not a string of length 4",
            ),
        ),
        (
            "%c",
            vec![(ItemKind::NoneValue, 0)],
            Err("format argument 1: %c requires an integer or a unicode character, not NoneType"),
        ),
        (
            "%*d",
            vec![float(1.0), float(2.0)],
            Err("format argument 1: * requires int, not float"),
        ),
        (
            "%\u{7}",
            vec![],
            Err("stray % at position 0 or unexpected format character U+0007 at position 1"),
        ),
        (
            "%\u{e9}",
            vec![],
            Err(
                "stray % at position 0 or unexpected format character '\u{e9}' (U+00E9) at position 1",
            ),
        ),
        (
            "%(a)s %s",
            vec![],
            Err("format requires a mapping, not tuple"),
        ),
    ]
}

#[test]
fn a_dict_gives_keyed_values_and_a_list_gives_the_items_of_a_tuple() {
    let word = |value: i64| inline(value).unwrap();
    let dict = crate::dict::typy_rt_dict_new(
        word(ItemKind::Object as i64),
        0,
        word(ItemKind::Int as i64),
        0,
        word(0),
    );
    // SAFETY: the dict is live and takes its own references.
    unsafe {
        crate::dict::typy_rt_dict_set(dict, text_word("a"), word(7).cast_unsigned());
        crate::dict::typy_rt_dict_set(dict, text_word("\u{3b1}"), word(8).cast_unsigned());
    }
    let mapping = [(ItemKind::Object, dict.expose_provenance() as u64)];
    for (format, expected) in [
        ("%(a)d-%(a)05.1f", Ok("7-007.0")),
        ("no keys", Ok("no keys")),
        ("%s", Ok("{'a': 7, '\u{3b1}': 8}")),
        ("%(b)s", Err("'b'")),
        (
            "%(a)s %d",
            Err("format requires a parenthesised mapping key at position 6"),
        ),
        (
            "%(a)*d",
            Err("* cannot be used with a parenthesised mapping key at position 0"),
        ),
        ("%(a)c", Ok("\u{7}")),
        ("%(\u{3b1})s", Ok("8")),
        (
            "%(a).*d",
            Err("* cannot be used with a parenthesised mapping key at position 0"),
        ),
    ] {
        let found = format_items(format, &mapping, 1);
        match expected {
            Ok(text) => assert_eq!(found.as_deref(), Ok(text), "{format}"),
            Err(message) => assert_eq!(found, Err(message.to_owned()), "{format}"),
        }
    }
    let list = crate::list::new_list(ItemKind::Int, 2);
    // SAFETY: the list is live and takes the items.
    unsafe {
        crate::list::push_retained(list, word(1).cast_unsigned());
        crate::list::push_retained(list, word(2).cast_unsigned());
    }
    let items = [(ItemKind::Object, list.expose_provenance() as u64)];
    assert_eq!(format_items("%d+%d", &items, 2).as_deref(), Ok("1+2"));
    assert_eq!(
        format_items("%d", &items, 2),
        Err("not all arguments converted during string formatting (required 1, got 2)".to_owned())
    );
    assert_eq!(
        format_items("%s %s", &[(ItemKind::Int, word(1).cast_unsigned())], 1),
        Err("not enough arguments for format string (got 1)".to_owned())
    );
    // SAFETY: the test holds the only references.
    unsafe {
        typy_rt_decrement(dict.cast());
        typy_rt_decrement(list.cast());
    }
}

#[test]
fn each_conversion_takes_the_kinds_of_value_cpython_takes() {
    let three = (ItemKind::Int, inline(3).unwrap().cast_unsigned());
    let truth = (ItemKind::Bool, 1);
    for (kinds, numbers, needs) in [
        ("diu", ["3", "1", "2"], "a real number"),
        ("xXo", ["3", "1", ""], "an integer"),
        (
            "eE",
            ["3.000000e+00", "1.000000e+00", "2.500000e+00"],
            "a real number",
        ),
        ("fF", ["3.000000", "1.000000", "2.500000"], "a real number"),
        ("gG", ["3", "1", "2.5"], "a real number"),
    ] {
        for kind in kinds.chars() {
            let format = format!("%{kind}");
            for (item, expected) in [three, truth, float(2.5)].into_iter().zip(numbers) {
                let found = format_items(&format, &[item], 0);
                if expected.is_empty() {
                    let message = format!("format argument 1: %{kind} requires {needs}, not float");
                    assert_eq!(found, Err(message));
                } else if kind.is_ascii_uppercase() {
                    assert_eq!(found, Ok(expected.to_uppercase()), "{format}");
                } else {
                    assert_eq!(found.as_deref(), Ok(expected), "{format}");
                }
            }
            let text = (ItemKind::Object, text_word("s"));
            assert_eq!(
                format_items(&format, &[text], 0),
                Err(format!(
                    "format argument 1: %{kind} requires {needs}, not str"
                ))
            );
        }
    }
}

#[test]
fn an_int_too_large_for_a_float_fails_a_float_conversion() {
    let big = crate::int::value::Integer {
        negative: false,
        magnitude: crate::int::natural::shift_left(&[1], 1100),
    }
    .into_word();
    assert_eq!(
        format_items("%f", &[(ItemKind::Int, big.cast_unsigned())], 0),
        Err("int too large to convert to float".to_owned())
    );
}

#[test]
fn a_key_missing_from_a_dict_of_other_keys_raises_key_error() {
    let word = |value: i64| inline(value).unwrap();
    let dict = crate::dict::typy_rt_dict_new(
        word(ItemKind::Int as i64),
        0,
        word(ItemKind::Int as i64),
        0,
        word(0),
    );
    // SAFETY: the dict is live.
    unsafe {
        crate::dict::typy_rt_dict_set(dict, word(1).cast_unsigned(), word(2).cast_unsigned());
    }
    let mapping = [(ItemKind::Object, dict.expose_provenance() as u64)];
    assert_eq!(format_items("%(a)s", &mapping, 1), Err("'a'".to_owned()));
    // SAFETY: the test holds the only reference.
    unsafe { typy_rt_decrement(dict.cast()) };
}

#[test]
fn a_width_or_a_precision_past_a_c_integer_is_refused_as_cpython_refuses_it() {
    let big = |value: i128| {
        (
            ItemKind::Int,
            crate::int::value::Integer::from_i128(value)
                .into_word()
                .cast_unsigned(),
        )
    };
    let one = (ItemKind::Int, inline(1).unwrap().cast_unsigned());
    let cases = [
        (
            "%*d",
            vec![big(1 << 70), one],
            "Python int too large to convert to C ssize_t",
        ),
        (
            "%*d",
            vec![big(-(1 << 70)), one],
            "Python int too large to convert to C ssize_t",
        ),
        (
            "%.*d",
            vec![big(1 << 31), one],
            "Python int too large to convert to C int",
        ),
        ("%99999999999999999999d", vec![one], "width too big"),
        ("%.3000000000d", vec![one], "precision too big"),
    ];
    for (format, items, expected) in cases {
        assert_eq!(
            format_items(format, &items, 0),
            Err(expected.to_owned()),
            "{format}"
        );
    }
}
