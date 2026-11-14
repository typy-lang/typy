//! `format % values` where the format is known only when the program runs (LIB-11): the
//! format is read here with the grammar of CPython's `unicode_format_arg_parse`, and each
//! conversion takes its value by its kind, with CPython 3.15's messages (OI-14).

use super::{Printf, char_str_text, char_text, float_text, int_text, str_text};
use crate::Raising;
use crate::exception::{ExceptionKind, ExceptionObject};
use crate::list::{ItemKind, ItemType, ListObject};
use crate::message::MessageBuffer;
use crate::string::{StrBuilder, StrObject};
use crate::tuple::TupleType;
use crate::word::{inline, inline_value};

/// How the values are given: the items of a tuple, one value, or the items of a
/// `tuple[T, ...]`, which is held as a list.
const TUPLE: i64 = 0;
const LIST: i64 = 2;

/// The values a format takes.
struct Values {
    /// The layout and address of the values given as a tuple, or of the one value as a
    /// tuple of one.
    layout: *const TupleType,
    address: u64,
    /// The list that holds a `tuple[T, ...]`'s items.
    list: *const ListObject,
    /// Whether the values are one value rather than a tuple of them.
    one: bool,
    /// The dict a key reads, when the one value is a dict.
    mapping: Option<*const crate::dict::DictObject>,
    next: usize,
}

impl Values {
    fn count(&self) -> usize {
        if !self.list.is_null() {
            // SAFETY: the list is live.
            return unsafe { (*self.list).length };
        }
        // SAFETY: the descriptor lives as long as the program.
        usize::try_from(unsafe { (*self.layout).length }).unwrap_or(0)
    }

    /// Returns value `index` with its type, as the value it holds when it is optional.
    fn item(&self, index: usize) -> (ItemType, u64) {
        // SAFETY: the index is below the count, and the values are live.
        let (ty, value) = unsafe {
            if self.list.is_null() {
                (
                    crate::tuple::field_type(self.layout, index),
                    crate::tuple::field(self.layout, self.address, index),
                )
            } else {
                ((*self.list).item_type(), (*self.list).item(index))
            }
        };
        // SAFETY: the value is live.
        unsafe { ty.held_value(value) }.unwrap_or((ItemType::from(ItemKind::NoneValue), 0))
    }

    /// Returns the next value with its position from 1, or CPython's error when there is none.
    fn next(&mut self) -> Result<(usize, ItemType, u64), *mut ExceptionObject> {
        if self.next >= self.count() {
            let mut message = MessageBuffer::new();
            message.push_str("not enough arguments for format string (got ");
            message.push_number(count_of(self.count()));
            message.push_str(")");
            return Err(message.raise(ExceptionKind::TypeError));
        }
        let (ty, value) = self.item(self.next);
        self.next += 1;
        Ok((self.next, ty, value))
    }
}

fn count_of(count: usize) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

/// Returns `format % values`. `form` is the `int` word of 0 when `values` is a tuple of
/// layout `values_type`, 1 when it is one value given as a tuple of one, and 2 when that one
/// value is a `tuple[T, ...]`, whose items are the values.
///
/// # Safety
///
/// `format` points to a live `str`, and `address` to a live tuple of `values_type`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_percent_dynamic(
    format: *const StrObject,
    values_type: *const TupleType,
    address: u64,
    form: i64,
) -> Raising<*mut StrObject> {
    let form = inline_value(form);
    let one = form != TUPLE;
    // One value is the only field of its tuple; a tuple of values may have none.
    // SAFETY: the caller's guarantee.
    let first = one.then(|| unsafe {
        (
            crate::tuple::field_type(values_type, 0),
            crate::tuple::field(values_type, address, 0),
        )
    });
    let list = match first {
        Some((_, value)) if form == LIST => {
            core::ptr::with_exposed_provenance::<ListObject>(usize::try_from(value).unwrap_or(0))
        }
        _ => core::ptr::null(),
    };
    // SAFETY: the one value is live.
    let mapping = first
        .filter(|_| form != LIST)
        .and_then(|(ty, value)| unsafe { ty.held_value(value) })
        .filter(|(ty, value)| ty.kind() == ItemKind::Object && is_dict(*value))
        .map(|(_, value)| core::ptr::with_exposed_provenance(usize::try_from(value).unwrap_or(0)));
    let mut values = Values {
        layout: values_type,
        address,
        list,
        one: one && form != LIST,
        mapping,
        next: 0,
    };
    // SAFETY: the caller passes a live format.
    match unsafe { formatted(format, &mut values) } {
        Ok(text) => Raising::ok(text),
        Err(error) => Raising::error(core::ptr::null_mut(), error),
    }
}

/// Returns whether an object is a dict.
fn is_dict(value: u64) -> bool {
    let object: *const crate::header::ObjectHeader =
        core::ptr::with_exposed_provenance(usize::try_from(value).unwrap_or(0));
    // SAFETY: the value is a live object.
    !object.is_null() && unsafe { (*object).type_id } == crate::header::DICT_TYPE_ID
}

/// Returns the text of the format with each conversion made.
///
/// # Safety
///
/// `format` points to a live `str`, and the values are live.
unsafe fn formatted(
    format: *const StrObject,
    values: &mut Values,
) -> Result<*mut StrObject, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    let points: StrBuilder = unsafe { super::points(format) };
    let mut out = StrBuilder::with_capacity(points.points().len());
    let result = write(points.points(), values, &mut out);
    points.discard();
    match result {
        Ok(()) => Ok(out.finish()),
        Err(error) => {
            out.discard();
            Err(error)
        }
    }
}

/// Writes the format to `out`, each conversion with its value, and checks that every value
/// was used.
fn write(
    points: &[u32],
    values: &mut Values,
    out: &mut StrBuilder,
) -> Result<(), *mut ExceptionObject> {
    let percent = u32::from(b'%');
    let mut at = 0;
    let mut keyed = false;
    while at < points.len() {
        if points[at] != percent {
            out.push(points[at]);
            at += 1;
            continue;
        }
        let position = at;
        at += 1;
        if points.get(at) == Some(&percent) {
            out.push(percent);
            at += 1;
            continue;
        }
        let conversion = parse(points, position, &mut at)?;
        let text = if let Some((start, end)) = conversion.key {
            keyed = true;
            keyed_text(&conversion, &points[start..end], values)?
        } else if keyed {
            return Err(value_error_at(
                "format requires a parenthesised mapping key at position ",
                position,
            ));
        } else {
            unnamed_text(&conversion, values)?
        };
        // SAFETY: the text is new; it is copied and released.
        unsafe {
            out.push_str(text);
            crate::refcount::typy_rt_decrement(text.cast());
        }
    }
    // CPython leaves a mapping the conversions do not use alone.
    if values.mapping.is_none() && values.next < values.count() {
        let mut message = MessageBuffer::new();
        message.push_str("not all arguments converted during string formatting (required ");
        message.push_number(count_of(values.next));
        message.push_str(", got ");
        message.push_number(count_of(values.count()));
        message.push_str(")");
        return Err(message.raise(ExceptionKind::TypeError));
    }
    Ok(())
}

/// A width or a precision as written.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Number {
    Absent,
    Given(usize),
    Star,
}

/// One conversion of the format.
struct Conversion {
    position: usize,
    /// Where the key is in the format, for `%(key)s`.
    key: Option<(usize, usize)>,
    printf: Printf,
    width: Number,
    precision: Number,
    kind: u32,
}

/// The conversion characters CPython accepts.
const KINDS: &[u8] = b"sradiuoxXeEfFgGc";

/// Reads one conversion after its `%`, leaving `at` after its conversion character.
fn parse(
    points: &[u32],
    position: usize,
    at: &mut usize,
) -> Result<Conversion, *mut ExceptionObject> {
    let is = |index: usize, wanted: u8| points.get(index) == Some(&u32::from(wanted));
    let mut key = None;
    if is(*at, b'(') {
        // A key ends at the `)` that closes its `(`, so it may hold parentheses.
        let mut depth = 1;
        let start = *at + 1;
        *at += 1;
        while depth > 0 {
            match points.get(*at).and_then(|point| u8::try_from(*point).ok()) {
                Some(b'(') => depth += 1,
                Some(b')') => depth -= 1,
                Some(_) => {}
                None if *at < points.len() => {}
                None => {
                    return Err(value_error_at(
                        "stray % or incomplete format key at position ",
                        position,
                    ));
                }
            }
            *at += 1;
        }
        key = Some((start, *at - 1));
    }
    let mut printf = Printf::default();
    while let Some(flag) = points.get(*at).and_then(|point| u8::try_from(*point).ok()) {
        match flag {
            b'-' => printf.left = true,
            b'+' => printf.plus = true,
            b' ' => printf.space = true,
            b'#' => printf.alternate = true,
            b'0' => printf.zero = true,
            _ => break,
        }
        *at += 1;
    }
    let width = number(points, at);
    let precision = if is(*at, b'.') {
        *at += 1;
        match number(points, at) {
            Number::Absent => Number::Given(0),
            given => given,
        }
    } else {
        Number::Absent
    };
    if points
        .get(*at)
        .is_some_and(|point| b"hlL".iter().any(|modifier| u32::from(*modifier) == *point))
    {
        *at += 1;
    }
    let Some(&kind) = points.get(*at) else {
        return Err(value_error_at("stray % at position ", position));
    };
    if !KINDS.iter().any(|known| u32::from(*known) == kind) {
        return Err(unexpected(kind, position, *at));
    }
    *at += 1;
    Ok(Conversion {
        position,
        key,
        printf,
        width,
        precision,
        kind,
    })
}

/// Reads `*` or decimal digits at `at`.
fn number(points: &[u32], at: &mut usize) -> Number {
    if points.get(*at) == Some(&u32::from(b'*')) {
        *at += 1;
        return Number::Star;
    }
    let start = *at;
    let mut value: usize = 0;
    while let Some(digit) = points
        .get(*at)
        .and_then(|point| char::from_u32(*point))
        .and_then(|character| character.to_digit(10))
    {
        value = value.saturating_mul(10).saturating_add(digit as usize);
        *at += 1;
    }
    if *at == start {
        Number::Absent
    } else {
        Number::Given(value)
    }
}

/// Returns `ValueError` with `message` and a position.
fn value_error_at(message: &str, position: usize) -> *mut ExceptionObject {
    let mut text = MessageBuffer::new();
    text.push_str(message);
    text.push_number(count_of(position));
    text.raise(ExceptionKind::ValueError)
}

/// Returns the `ValueError` for a conversion character CPython does not have: a letter names
/// the conversion, and any other character is shown with where it is.
fn unexpected(kind: u32, position: usize, at: usize) -> *mut ExceptionObject {
    let character = char::from_u32(kind).unwrap_or('\u{fffd}');
    let mut text = MessageBuffer::new();
    if character.is_ascii_alphabetic() {
        text.push_str("unsupported format %");
        text.push(kind);
        text.push_str(" at position ");
        text.push_number(count_of(position));
        return text.raise(ExceptionKind::ValueError);
    }
    text.push_str("stray % at position ");
    text.push_number(count_of(position));
    text.push_str(" or unexpected format character ");
    if character.is_control() {
        text.push_str("U+");
        text.push_hex(kind, 4);
    } else {
        text.push_str("'");
        text.push(kind);
        text.push_str("'");
        if !character.is_ascii() {
            text.push_str(" (U+");
            text.push_hex(kind, 4);
            text.push_str(")");
        }
    }
    text.push_str(" at position ");
    text.push_number(count_of(at));
    text.raise(ExceptionKind::ValueError)
}

/// Returns a conversion without a key, which takes the next values in order.
fn unnamed_text(
    conversion: &Conversion,
    values: &mut Values,
) -> Result<*mut StrObject, *mut ExceptionObject> {
    let mut printf = conversion.printf;
    if conversion.width == Number::Star {
        let width = star(values)?;
        if width < 0 {
            printf.left = true;
        }
        printf.width = usize::try_from(width.unsigned_abs()).unwrap_or(usize::MAX);
    } else if let Number::Given(width) = conversion.width {
        printf.width = width;
    }
    match conversion.precision {
        Number::Star => {
            let precision = star(values)?;
            printf.precision = Some(usize::try_from(precision.max(0)).unwrap_or(usize::MAX));
        }
        Number::Given(precision) => printf.precision = Some(precision),
        Number::Absent => {}
    }
    let (position, ty, value) = values.next()?;
    let mut label = MessageBuffer::new();
    label.push_str("format argument");
    if !values.one {
        label.push_str(" ");
        label.push_number(count_of(position));
    }
    convert(printf, conversion.kind, ty, value, &label)
}

/// Returns the `int` a `*` takes from the next value.
fn star(values: &mut Values) -> Result<i64, *mut ExceptionObject> {
    let (position, ty, value) = values.next()?;
    let number = match ty.kind() {
        ItemKind::Int if crate::word::is_inline(value.cast_signed()) => {
            inline_value(value.cast_signed())
        }
        ItemKind::Int => i64::MAX,
        ItemKind::Bool => i64::from(value != 0),
        _ => {
            let mut message = MessageBuffer::new();
            message.push_str("format argument ");
            message.push_number(count_of(position));
            message.push_str(": * requires int, not ");
            message.push_str(class_name(ty, value));
            return Err(message.raise(ExceptionKind::TypeError));
        }
    };
    Ok(number)
}

/// Returns a conversion with a key, `%(key)s`, whose value comes from the dict.
fn keyed_text(
    conversion: &Conversion,
    key: &[u32],
    values: &Values,
) -> Result<*mut StrObject, *mut ExceptionObject> {
    let Some(mapping) = values.mapping else {
        let mut message = MessageBuffer::new();
        message.push_str("format requires a mapping, not ");
        // A tuple of values may have none to read.
        message.push_str(if values.one {
            let (ty, value) = values.item(0);
            class_name(ty, value)
        } else {
            "tuple"
        });
        return Err(message.raise(ExceptionKind::TypeError));
    };
    if conversion.width == Number::Star || conversion.precision == Number::Star {
        return Err(value_error_at(
            "* cannot be used with a parenthesised mapping key at position ",
            conversion.position,
        ));
    }
    let mut printf = conversion.printf;
    if let Number::Given(width) = conversion.width {
        printf.width = width;
    }
    if let Number::Given(precision) = conversion.precision {
        printf.precision = Some(precision);
    }
    let mut text = StrBuilder::with_capacity(key.len());
    for point in key {
        text.push(*point);
    }
    let key = text.finish();
    // SAFETY: the dict is live, and the key a new `str`.
    let found = unsafe { lookup(mapping, key) };
    let result = found.and_then(|(ty, value)| {
        let mut label = MessageBuffer::new();
        label.push_str("format argument ");
        // SAFETY: the key is a live `str`.
        unsafe { label.push_repr(key) };
        convert(printf, conversion.kind, ty, value, &label)
    });
    // SAFETY: the key is no longer used.
    unsafe { crate::refcount::typy_rt_decrement(key.cast()) };
    result
}

/// Returns the value of `key` in the dict with its type, or the `KeyError` CPython raises.
///
/// # Safety
///
/// `dict` points to a live dict, and `key` to a live `str`.
unsafe fn lookup(
    dict: *const crate::dict::DictObject,
    key: *mut StrObject,
) -> Result<(ItemType, u64), *mut ExceptionObject> {
    let bits = key.expose_provenance() as u64;
    let text = ItemType::from(ItemKind::Object);
    // SAFETY: the caller's guarantee.
    unsafe {
        let key_type = (*dict).key_type();
        let found = if key_type.kind() == ItemKind::Object {
            crate::dict::find(dict, bits, key_type.hash(bits))
        } else {
            None
        };
        match found {
            Some(position) => {
                let ty = (*dict).value_type();
                let value = (*dict).value(position);
                Ok(ty
                    .held_value(value)
                    .unwrap_or((ItemType::from(ItemKind::NoneValue), 0)))
            }
            None => Err(crate::set::key_error(text, bits)),
        }
    }
}

/// Returns the name of a value's class, as CPython's messages write it.
fn class_name(ty: ItemType, value: u64) -> &'static str {
    match ty.kind() {
        ItemKind::Int => "int",
        ItemKind::Float => "float",
        ItemKind::Bool => "bool",
        ItemKind::NoneValue => "NoneType",
        ItemKind::Tuple => "tuple",
        _ => {
            let object: *const crate::header::ObjectHeader =
                core::ptr::with_exposed_provenance(usize::try_from(value).unwrap_or(0));
            // SAFETY: the value is a live object.
            unsafe { crate::type_object::type_of(object) }.name()
        }
    }
}

/// Returns whether a value is a `str`.
fn is_str(ty: ItemType, value: u64) -> bool {
    ty.kind() == ItemKind::Object && class_name(ty, value) == "str"
}

/// Returns one conversion of a value laid out by `printf`; `label` names the value in an
/// error.
fn convert(
    printf: Printf,
    kind: u32,
    ty: ItemType,
    value: u64,
    label: &MessageBuffer,
) -> Result<*mut StrObject, *mut ExceptionObject> {
    let kind = u8::try_from(kind).unwrap_or(b's');
    let integer = matches!(ty.kind(), ItemKind::Int | ItemKind::Bool);
    let as_int = || {
        if ty.kind() == ItemKind::Bool {
            inline(i64::from(value != 0)).unwrap_or(1)
        } else {
            value.cast_signed()
        }
    };
    // SAFETY: each value is a live item of its type.
    let result = unsafe {
        match kind {
            b's' | b'r' | b'a' => {
                let written = if kind == b's' {
                    ty.str(value)
                } else {
                    ty.repr(value)
                };
                if !written.exception.is_null() {
                    return Err(written.exception);
                }
                let text = if kind == b'a' {
                    let escaped = crate::format::typy_rt_ascii(written.value);
                    crate::refcount::typy_rt_decrement(written.value.cast());
                    escaped
                } else {
                    written.value
                };
                let laid = str_text(printf, text);
                crate::refcount::typy_rt_decrement(text.cast());
                laid
            }
            b'd' | b'i' | b'u' if integer => int_text(printf, as_int(), b'd'),
            b'd' | b'i' | b'u' if ty.kind() == ItemKind::Float => {
                let whole = crate::float::typy_rt_float_to_int(f64::from_bits(value));
                if !whole.exception.is_null() {
                    return Err(whole.exception);
                }
                let text = int_text(printf, whole.value, b'd');
                crate::refcount::typy_rt_int_decrement(whole.value);
                text
            }
            b'x' | b'X' | b'o' if integer => int_text(printf, as_int(), kind),
            b'c' if integer => with_label(label, |label| char_text(printf, as_int(), label)),
            b'c' if is_str(ty, value) => with_label(label, |label| {
                char_str_text(
                    printf,
                    core::ptr::with_exposed_provenance(usize::try_from(value).unwrap_or(0)),
                    label,
                )
            }),
            b'e' | b'E' | b'f' | b'F' | b'g' | b'G' if integer => {
                let real = crate::float::typy_rt_int_to_float(as_int());
                if !real.exception.is_null() {
                    return Err(real.exception);
                }
                float_text(printf, real.value, kind)
            }
            b'e' | b'E' | b'f' | b'F' | b'g' | b'G' if ty.kind() == ItemKind::Float => {
                float_text(printf, f64::from_bits(value), kind)
            }
            _ => {
                let needs = match kind {
                    b'x' | b'X' | b'o' => "an integer",
                    b'c' => "an integer or a unicode character",
                    _ => "a real number",
                };
                let mut message = MessageBuffer::new();
                message.push_buffer(label);
                message.push_str(": %");
                message.push(u32::from(kind));
                message.push_str(" requires ");
                message.push_str(needs);
                message.push_str(", not ");
                message.push_str(class_name(ty, value));
                return Err(message.raise(ExceptionKind::TypeError));
            }
        }
    };
    if result.exception.is_null() {
        Ok(result.value)
    } else {
        Err(result.exception)
    }
}

/// Calls `make` with the label as a new `str`, which is released after.
fn with_label(
    label: &MessageBuffer,
    make: impl FnOnce(*const StrObject) -> Raising<*mut StrObject>,
) -> Raising<*mut StrObject> {
    let text = label.text();
    let result = make(text);
    // SAFETY: the label is new and no longer used.
    unsafe { crate::refcount::typy_rt_decrement(text.cast()) };
    result
}

#[cfg(test)]
mod tests {
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
                Err(
                    "format argument 1: %c requires an integer or a unicode character, not NoneType",
                ),
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
        }
        let mapping = [(ItemKind::Object, dict.expose_provenance() as u64)];
        for (format, expected) in [
            ("%(a)d-%(a)05.1f", Ok("7-007.0")),
            ("no keys", Ok("no keys")),
            ("%s", Ok("{'a': 7}")),
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
            Err(
                "not all arguments converted during string formatting (required 1, got 2)"
                    .to_owned()
            )
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
}
