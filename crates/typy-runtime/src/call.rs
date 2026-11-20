//! Binding the arguments of a call that unpacks with `*` or `**` when the call runs (EXP-17,
//! ADR-0073): the keyword names a call gives, in source order, are checked against the
//! callee's parameters as CPython binds them, with CPython 3.15's `TypeError`.
//!
//! The compiler describes the callee in a `str` of lines: the qualified name binding errors
//! give, the name with its module a keyword given twice gives, then one line for each
//! parameter: its kind, `P` positional-only, `N` positional or keyword, `A` `*args`, `K`
//! keyword-only or `W` `**kwargs`, then `=` when it has a default or `-`, then its name.

#[cfg(test)]
mod tests;

use crate::dict::{DictObject, typy_rt_dict_contains, typy_rt_dict_set};
use crate::exception::{ExceptionKind, ExceptionObject};
use crate::message::MessageBuffer;
use crate::string::{StrObject, code_point, length};
use crate::word::{inline, inline_value};

const NEWLINE: u32 = 0x0a;
const POSITIONAL_ONLY: u32 = b'P' as u32;
const EITHER: u32 = b'N' as u32;
const STAR_ARGS: u32 = b'A' as u32;
const KEYWORD_ONLY: u32 = b'K' as u32;
const STAR_STAR_KWARGS: u32 = b'W' as u32;
const DEFAULT: u32 = b'=' as u32;

/// The cost of a deletion, an insertion or a substitution of one byte in CPython's
/// suggestions, and of a change of ASCII case.
const MOVE_COST: usize = 2;
const CASE_COST: usize = 1;
/// The longest names CPython compares, in UTF-8 bytes, after their common affixes.
const MAX_STRING_SIZE: usize = 40;
/// The most candidates CPython compares a name with.
const MAX_CANDIDATE_ITEMS: usize = 750;
/// The longest name compared here, in UTF-8 bytes; a longer one gets no suggestion.
const MAX_NAME_BYTES: usize = 256;

/// Some code points of a live `str`.
#[derive(Clone, Copy)]
struct Text {
    string: *const StrObject,
    start: usize,
    end: usize,
}

impl Text {
    /// Returns the whole of `string`.
    ///
    /// # Safety
    ///
    /// `string` points to a live `str` object.
    unsafe fn of(string: *const StrObject) -> Self {
        Self {
            string,
            start: 0,
            // SAFETY: the caller's guarantee.
            end: unsafe { length(string) },
        }
    }

    const fn len(self) -> usize {
        self.end - self.start
    }

    /// Returns the code point at `index`, below the length.
    fn point(self, index: usize) -> u32 {
        // SAFETY: `string` is live while the call runs, and the index is in bounds.
        unsafe { code_point(self.string, self.start + index) }
    }

    /// Returns true when both hold the same code points.
    fn equals(self, other: Self) -> bool {
        self.len() == other.len()
            && (0..self.len()).all(|index| self.point(index) == other.point(index))
    }

    /// Appends the code points to `message`.
    fn push_into(self, message: &mut MessageBuffer) {
        for index in 0..self.len() {
            message.push(self.point(index));
        }
    }

    /// Writes the UTF-8 bytes into `buffer` and returns how many there are, or `None` when
    /// they do not fit.
    fn utf8(self, buffer: &mut [u8; MAX_NAME_BYTES]) -> Option<usize> {
        let mut used = 0;
        for index in 0..self.len() {
            let point = char::from_u32(self.point(index))?;
            let width = point.len_utf8();
            if used + width > buffer.len() {
                return None;
            }
            point.encode_utf8(&mut buffer[used..used + width]);
            used += width;
        }
        Some(used)
    }
}

/// One parameter of the callee.
#[derive(Clone, Copy)]
struct Parameter {
    kind: u32,
    default: bool,
    name: Text,
}

impl Parameter {
    /// Returns true for a parameter a positional argument fills.
    const fn positional(self) -> bool {
        self.kind == POSITIONAL_ONLY || self.kind == EITHER
    }

    /// Returns true for a parameter a keyword argument fills.
    const fn named(self) -> bool {
        self.kind == EITHER || self.kind == KEYWORD_ONLY
    }
}

/// A call's description of its callee.
struct Description {
    /// The qualified name, such as `Box.put`.
    name: Text,
    /// The name with its module, such as `__main__.Box.put`.
    full_name: Text,
    /// The parameter lines.
    parameters: Text,
}

impl Description {
    /// Reads the description `string`.
    ///
    /// # Safety
    ///
    /// `string` points to a live `str` object the compiler wrote as the module documentation
    /// says.
    unsafe fn read(string: *const StrObject) -> Self {
        // SAFETY: the caller's guarantee.
        let whole = unsafe { Text::of(string) };
        let mut lines = Lines { text: whole };
        let name = lines.next().unwrap_or(whole);
        let full_name = lines.next().unwrap_or(name);
        Self {
            name,
            full_name,
            parameters: lines.text,
        }
    }

    /// Returns the parameters, in order.
    fn parameters(&self) -> impl Iterator<Item = Parameter> {
        Lines {
            text: self.parameters,
        }
        .filter(|line| line.len() > 2)
        .map(|line| Parameter {
            kind: line.point(0),
            default: line.point(1) == DEFAULT,
            name: Text {
                start: line.start + 2,
                ..line
            },
        })
    }

    /// Returns true when a parameter is of `kind`.
    fn has(&self, kind: u32) -> bool {
        self.parameters().any(|parameter| parameter.kind == kind)
    }

    /// Returns the parameter a keyword `name` fills, with its index, if any.
    fn named(&self, name: Text) -> Option<(usize, Parameter)> {
        self.parameters()
            .enumerate()
            .find(|(_, parameter)| parameter.named() && parameter.name.equals(name))
    }
}

/// The lines of some text.
struct Lines {
    text: Text,
}

impl Iterator for Lines {
    type Item = Text;

    fn next(&mut self) -> Option<Text> {
        if self.text.len() == 0 {
            return None;
        }
        let end = (0..self.text.len())
            .find(|index| self.text.point(*index) == NEWLINE)
            .map_or(self.text.end, |index| self.text.start + index);
        let line = Text { end, ..self.text };
        self.text.start = (end + 1).min(self.text.end);
        Some(line)
    }
}

/// Returns the keys of `names` in order, or none for null.
///
/// # Safety
///
/// `names` is null or points to a live dict whose keys are `str` objects.
unsafe fn keys(names: *const DictObject) -> impl Iterator<Item = Text> {
    let filled = if names.is_null() {
        0
    } else {
        // SAFETY: the caller's guarantee.
        unsafe { (*names).filled }
    };
    (0..filled).filter_map(move |index| {
        // SAFETY: `index` is below `filled`, and a live entry holds a live key.
        unsafe {
            (*names).is_live(index).then(|| {
                let key = (*names).key(index);
                Text::of(core::ptr::with_exposed_provenance(
                    usize::try_from(key).unwrap_or(0),
                ))
            })
        }
    })
}

/// Records the keyword argument `name` in `names`, the keyword names of the call so far, or
/// returns CPython's `TypeError` when an earlier argument gave it.
///
/// # Safety
///
/// `description` is a live description, `names` a live dict of `str` keys and `int` values,
/// and `name` a live `str`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_call_name(
    description: *const StrObject,
    names: *mut DictObject,
    name: *const StrObject,
) -> *mut ExceptionObject {
    let key = name.expose_provenance() as u64;
    // SAFETY: the caller's guarantee.
    unsafe {
        if typy_rt_dict_contains(names, key) {
            return repeated(&Description::read(description), Text::of(name));
        }
        typy_rt_dict_set(names, key, zero());
    }
    core::ptr::null_mut()
}

/// Merges the entries of `source`, the dict of a `**` argument, into `merged`, records each
/// key in `names`, and puts each entry whose key no parameter takes by name into `kwargs`,
/// the callee's `**kwargs`, unless it is 0. Returns CPython's `TypeError` for a key an
/// earlier argument gave, after merging the entries before it.
///
/// # Safety
///
/// `description` is a live description; `names` is a live dict of `str` keys and `int`
/// values; `merged` and `source` are live dicts of `str` keys and values held alike; `kwargs`
/// is 0 or the address of a live dict like them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_call_merge(
    description: *const StrObject,
    names: *mut DictObject,
    merged: *mut DictObject,
    source: *const DictObject,
    kwargs: u64,
) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee; `index` stays below `filled`.
    unsafe {
        let description = Description::read(description);
        let kwargs = core::ptr::with_exposed_provenance_mut::<DictObject>(
            usize::try_from(kwargs).unwrap_or(0),
        );
        for index in 0..(*source).filled {
            if !(*source).is_live(index) {
                continue;
            }
            let (key, value) = ((*source).key(index), (*source).value(index));
            let name = Text::of(core::ptr::with_exposed_provenance(
                usize::try_from(key).unwrap_or(0),
            ));
            if typy_rt_dict_contains(names, key) {
                return repeated(&description, name);
            }
            typy_rt_dict_set(names, key, zero());
            typy_rt_dict_set(merged, key, value);
            if !kwargs.is_null() && description.named(name).is_none() {
                typy_rt_dict_set(kwargs, key, value);
            }
        }
    }
    core::ptr::null_mut()
}

/// Returns CPython's `TypeError` when `positional` positional values, an `int` word, and the
/// keyword names in `names`, 0 for none, do not bind to the callee's parameters, checked in
/// CPython's order: each keyword, then the number of positional values, then the missing
/// positional and keyword-only arguments; null when they bind.
///
/// # Safety
///
/// `description` is a live description, and `names` is 0 or the address of a live dict of
/// `str` keys.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_call_check(
    description: *const StrObject,
    positional: i64,
    names: u64,
) -> *mut ExceptionObject {
    let given = usize::try_from(inline_value(positional)).unwrap_or(0);
    let names =
        core::ptr::with_exposed_provenance::<DictObject>(usize::try_from(names).unwrap_or(0));
    // SAFETY: the caller's guarantee.
    let description = unsafe { Description::read(description) };
    // SAFETY: the caller's guarantee.
    for name in unsafe { keys(names) } {
        match description.named(name) {
            Some((index, parameter)) if parameter.positional() && index < given => {
                let mut message = MessageBuffer::new();
                description.name.push_into(&mut message);
                message.push_str("() got multiple values for argument '");
                name.push_into(&mut message);
                message.push_str("'");
                return message.raise(ExceptionKind::TypeError);
            }
            Some(_) => {}
            None if description.has(STAR_STAR_KWARGS) => {}
            // SAFETY: the caller's guarantee.
            None => return unsafe { unexpected(&description, names, name) },
        }
    }
    let is_named = |parameter: Parameter| {
        // SAFETY: the caller's guarantee.
        unsafe { keys(names) }.any(|name| parameter.name.equals(name))
    };
    let positional_count = description
        .parameters()
        .filter(|each| each.positional())
        .count();
    if given > positional_count && !description.has(STAR_ARGS) {
        let keyword_only = description
            .parameters()
            .filter(|each| each.kind == KEYWORD_ONLY && is_named(*each))
            .count();
        return too_many(&description, given, keyword_only);
    }
    let missing_positional = |(index, parameter): &(usize, Parameter)| {
        parameter.positional() && *index >= given && !parameter.default && !is_named(*parameter)
    };
    let missing_keyword_only = |(_, parameter): &(usize, Parameter)| {
        parameter.kind == KEYWORD_ONLY && !parameter.default && !is_named(*parameter)
    };
    let checks: [(&str, MissingTest<'_>); 2] = [
        ("positional", &missing_positional),
        ("keyword-only", &missing_keyword_only),
    ];
    for (kind, missing) in checks {
        let count = description.parameters().enumerate().filter(missing).count();
        if count > 0 {
            let missing_names = description
                .parameters()
                .enumerate()
                .filter(missing)
                .map(|(_, parameter)| parameter.name);
            return missing_arguments(&description, kind, count, missing_names);
        }
    }
    core::ptr::null_mut()
}

/// A test of whether a parameter, with its index, misses its argument.
type MissingTest<'test> = &'test dyn Fn(&(usize, Parameter)) -> bool;

/// Returns the `int` word 0, the value `names` holds for each key.
fn zero() -> u64 {
    inline(0).unwrap_or(1).cast_unsigned()
}

/// Returns CPython's `TypeError` for a keyword name given twice.
fn repeated(description: &Description, name: Text) -> *mut ExceptionObject {
    let mut message = MessageBuffer::new();
    description.full_name.push_into(&mut message);
    message.push_str("() got multiple values for keyword argument '");
    name.push_into(&mut message);
    message.push_str("'");
    message.raise(ExceptionKind::TypeError)
}

/// Returns CPython's `TypeError` for the keyword `name` no parameter takes: the names of the
/// positional-only parameters the keywords give, when there are any, else the name and the
/// parameter it may be a misspelling of.
///
/// # Safety
///
/// `names` is null or points to a live dict of `str` keys.
unsafe fn unexpected(
    description: &Description,
    names: *const DictObject,
    name: Text,
) -> *mut ExceptionObject {
    let mut message = MessageBuffer::new();
    description.name.push_into(&mut message);
    let mut conflicts = description
        .parameters()
        .filter(|parameter| parameter.kind == POSITIONAL_ONLY)
        // SAFETY: the caller's guarantee.
        .filter(|parameter| unsafe { keys(names) }.any(|each| parameter.name.equals(each)))
        .peekable();
    if conflicts.peek().is_some() {
        message.push_str("() got some positional-only arguments passed as keyword arguments: '");
        for (position, parameter) in conflicts.enumerate() {
            if position > 0 {
                message.push_str(", ");
            }
            parameter.name.push_into(&mut message);
        }
        message.push_str("'");
        return message.raise(ExceptionKind::TypeError);
    }
    message.push_str("() got an unexpected keyword argument '");
    name.push_into(&mut message);
    message.push_str("'");
    if let Some(suggestion) = suggestion(description, name) {
        message.push_str(". Did you mean '");
        suggestion.push_into(&mut message);
        message.push_str("'?");
    }
    message.raise(ExceptionKind::TypeError)
}

/// Returns CPython's `TypeError` for `given` positional values, more than the parameters
/// take, when keywords fill `keyword_only` keyword-only parameters.
fn too_many(description: &Description, given: usize, keyword_only: usize) -> *mut ExceptionObject {
    let positional = description.parameters().filter(|each| each.positional());
    let (count, defaults) = positional.fold((0, 0), |(count, defaults), each| {
        (count + 1, defaults + usize::from(each.default))
    });
    let mut message = MessageBuffer::new();
    description.name.push_into(&mut message);
    message.push_str("() takes ");
    let plural = if defaults > 0 {
        message.push_str("from ");
        message.push_number(number(count - defaults));
        message.push_str(" to ");
        message.push_number(number(count));
        true
    } else {
        message.push_number(number(count));
        count != 1
    };
    message.push_str(if plural {
        " positional arguments but "
    } else {
        " positional argument but "
    });
    message.push_number(number(given));
    if keyword_only > 0 {
        message.push_str(if given == 1 {
            " positional argument (and "
        } else {
            " positional arguments (and "
        });
        message.push_number(number(keyword_only));
        message.push_str(if keyword_only == 1 {
            " keyword-only argument)"
        } else {
            " keyword-only arguments)"
        });
    }
    message.push_str(if given == 1 && keyword_only == 0 {
        " was given"
    } else {
        " were given"
    });
    message.raise(ExceptionKind::TypeError)
}

/// Returns CPython's `TypeError` for the `count` required arguments of `kind`, positional or
/// keyword-only, that no argument gives, each named in `names`.
fn missing_arguments(
    description: &Description,
    kind: &str,
    count: usize,
    names: impl Iterator<Item = Text>,
) -> *mut ExceptionObject {
    let mut message = MessageBuffer::new();
    description.name.push_into(&mut message);
    message.push_str("() missing ");
    message.push_number(number(count));
    message.push_str(" required ");
    message.push_str(kind);
    message.push_str(if count == 1 {
        " argument: "
    } else {
        " arguments: "
    });
    for (position, name) in names.enumerate() {
        // CPython joins two names with "and", and more with commas and a final ", and".
        if position > 0 && count == 2 {
            message.push_str(" and ");
        } else if position > 0 && position + 1 == count {
            message.push_str(", and ");
        } else if position > 0 {
            message.push_str(", ");
        }
        message.push_str("'");
        name.push_into(&mut message);
        message.push_str("'");
    }
    message.raise(ExceptionKind::TypeError)
}

fn number(count: usize) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

/// Returns the parameter a keyword takes whose name is nearest `name`, as CPython 3.15's
/// `_Py_CalculateSuggestions` finds it: among those a keyword fills, the first at the least
/// weighted edit distance of their UTF-8 bytes, if that is at most a third of their lengths.
fn suggestion(description: &Description, name: Text) -> Option<Text> {
    if description.parameters().filter(|each| each.named()).count() >= MAX_CANDIDATE_ITEMS {
        return None;
    }
    let candidates = description.parameters().filter(|each| each.named());
    let mut wanted = [0; MAX_NAME_BYTES];
    let wanted_size = name.utf8(&mut wanted)?;
    let mut best: Option<(Text, usize)> = None;
    for candidate in candidates {
        if candidate.name.equals(name) {
            continue;
        }
        let mut item = [0; MAX_NAME_BYTES];
        let Some(item_size) = candidate.name.utf8(&mut item) else {
            continue;
        };
        let mut max_distance = (wanted_size + item_size + 3) * MOVE_COST / 6;
        if let Some((_, distance)) = best {
            max_distance = max_distance.min(distance.saturating_sub(1));
        }
        let distance = levenshtein(&wanted[..wanted_size], &item[..item_size], max_distance);
        if distance > max_distance {
            continue;
        }
        if best.is_none_or(|(_, least)| distance < least) {
            best = Some((candidate.name, distance));
        }
    }
    best.map(|(text, _)| text)
}

/// Returns CPython's cost of changing byte `a` into byte `b`: nothing for the same byte, a
/// little for a change of ASCII case, a move otherwise.
const fn substitution_cost(a: u8, b: u8) -> usize {
    if a & 31 != b & 31 {
        return MOVE_COST;
    }
    if a == b {
        return 0;
    }
    if a.eq_ignore_ascii_case(&b) {
        CASE_COST
    } else {
        MOVE_COST
    }
}

/// Returns CPython's weighted Levenshtein distance between `a` and `b`, or more than
/// `max_cost` once it is sure to exceed it.
fn levenshtein<'text>(mut a: &'text [u8], mut b: &'text [u8], max_cost: usize) -> usize {
    while let (Some(first), Some(other)) = (a.first(), b.first())
        && first == other
    {
        (a, b) = (&a[1..], &b[1..]);
    }
    while let (Some(last), Some(other)) = (a.last(), b.last())
        && last == other
    {
        (a, b) = (&a[..a.len() - 1], &b[..b.len() - 1]);
    }
    if a.is_empty() || b.is_empty() {
        return (a.len() + b.len()) * MOVE_COST;
    }
    if a.len() > MAX_STRING_SIZE || b.len() > MAX_STRING_SIZE {
        return max_cost + 1;
    }
    if b.len() < a.len() {
        (a, b) = (b, a);
    }
    if (b.len() - a.len()) * MOVE_COST > max_cost {
        return max_cost + 1;
    }
    // One row of the matrix, updated in place: the cost of `b[..b_index]` into `a[..=index]`.
    let mut row = [0; MAX_STRING_SIZE];
    for (index, cost) in row.iter_mut().take(a.len()).enumerate() {
        *cost = (index + 1) * MOVE_COST;
    }
    let mut result = 0;
    for (b_index, code) in b.iter().enumerate() {
        let mut distance = b_index * MOVE_COST;
        result = distance;
        let mut minimum = usize::MAX;
        for (index, byte) in a.iter().enumerate() {
            let substitute = distance + substitution_cost(*code, *byte);
            distance = row[index];
            result = (result.min(distance) + MOVE_COST).min(substitute);
            row[index] = result;
            minimum = minimum.min(result);
        }
        if minimum > max_cost {
            return max_cost + 1;
        }
    }
    result
}
