//! Reads a printf-style format string, as `str % values` uses it (LIB-10), with the grammar
//! of CPython's `unicode_format_arg_parse` and the `ValueError` messages of CPython 3.15, the
//! newest wording (OI-14).

/// One piece of a format: literal text, or one conversion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Piece {
    /// Text written as it is; `%%` is one `%`.
    Literal(String),
    /// A conversion such as `%-8.3f`.
    Conversion(Conversion),
}

/// One conversion of a format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Conversion {
    /// Where its `%` is, in code points from the start of the format.
    pub(crate) position: usize,
    /// `%(key)s` takes its value from a mapping.
    pub(crate) key: Option<String>,
    /// The flags as written, from `-+ #0`.
    pub(crate) flags: String,
    pub(crate) width: Number,
    pub(crate) precision: Number,
    /// The conversion character, one of `sradiuoxXeEfFgGc`.
    pub(crate) kind: char,
}

/// A width or a precision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Number {
    Absent,
    Given(usize),
    /// `*`: the next value gives it.
    Star,
}

/// The conversion characters CPython accepts.
const KINDS: &str = "sradiuoxXeEfFgGc";

/// Returns the pieces of `format`, or the message of the `ValueError` CPython raises for it.
pub(crate) fn parse(format: &str) -> Result<Vec<Piece>, String> {
    let characters: Vec<char> = format.chars().collect();
    let mut pieces = Vec::new();
    let mut literal = String::new();
    let mut at = 0;
    while at < characters.len() {
        if characters[at] != '%' {
            literal.push(characters[at]);
            at += 1;
            continue;
        }
        let position = at;
        at += 1;
        if characters.get(at) == Some(&'%') {
            literal.push('%');
            at += 1;
            continue;
        }
        let conversion = conversion(&characters, position, &mut at)?;
        if !literal.is_empty() {
            pieces.push(Piece::Literal(std::mem::take(&mut literal)));
        }
        pieces.push(Piece::Conversion(conversion));
    }
    if !literal.is_empty() {
        pieces.push(Piece::Literal(literal));
    }
    Ok(pieces)
}

/// Reads one conversion after its `%`, leaving `at` after its conversion character.
fn conversion(characters: &[char], position: usize, at: &mut usize) -> Result<Conversion, String> {
    let incomplete = || format!("stray % at position {position}");
    let mut key = None;
    if characters.get(*at) == Some(&'(') {
        // A key ends at the `)` that closes its `(`, so it may hold parentheses.
        let mut depth = 1;
        let start = *at + 1;
        *at += 1;
        while depth > 0 {
            match characters.get(*at) {
                Some('(') => depth += 1,
                Some(')') => depth -= 1,
                Some(_) => {}
                None => {
                    return Err(format!(
                        "stray % or incomplete format key at position {position}"
                    ));
                }
            }
            *at += 1;
        }
        key = Some(characters[start..*at - 1].iter().collect());
    }
    let mut flags = String::new();
    while let Some(flag) = characters.get(*at).filter(|flag| "-+ #0".contains(**flag)) {
        flags.push(*flag);
        *at += 1;
    }
    let width = number(characters, at);
    // CPython keeps a width in a C `ssize_t` and a precision in a C `int`.
    if matches!(width, Number::Given(digits) if u64::try_from(digits).map_or(true, |digits| digits > i64::MAX.unsigned_abs()))
    {
        return Err("width too big".to_owned());
    }
    let precision = if characters.get(*at) == Some(&'.') {
        *at += 1;
        match number(characters, at) {
            Number::Absent => Number::Given(0),
            given => given,
        }
    } else {
        Number::Absent
    };
    if matches!(precision, Number::Given(digits) if digits > i32::MAX.unsigned_abs() as usize) {
        return Err("precision too big".to_owned());
    }
    if characters
        .get(*at)
        .is_some_and(|modifier| "hlL".contains(*modifier))
    {
        *at += 1;
    }
    let kind = *characters.get(*at).ok_or_else(incomplete)?;
    if !KINDS.contains(kind) {
        return Err(unexpected(kind, position, *at));
    }
    *at += 1;
    Ok(Conversion {
        position,
        key,
        flags,
        width,
        precision,
        kind,
    })
}

/// Returns the message for a conversion character CPython does not have: a letter names the
/// conversion, and any other character is shown with where it is.
fn unexpected(kind: char, position: usize, at: usize) -> String {
    if kind.is_ascii_alphabetic() {
        return format!("unsupported format %{kind} at position {position}");
    }
    let shown = if kind.is_control() {
        format!("U+{:04X}", u32::from(kind))
    } else if kind.is_ascii() {
        format!("'{kind}'")
    } else {
        format!("'{kind}' (U+{:04X})", u32::from(kind))
    };
    format!(
        "stray % at position {position} or unexpected format character {shown} at position {at}"
    )
}

/// Reads `*` or decimal digits at `at`.
fn number(characters: &[char], at: &mut usize) -> Number {
    if characters.get(*at) == Some(&'*') {
        *at += 1;
        return Number::Star;
    }
    let start = *at;
    while characters.get(*at).is_some_and(char::is_ascii_digit) {
        *at += 1;
    }
    if *at == start {
        return Number::Absent;
    }
    let digits: String = characters[start..*at].iter().collect();
    Number::Given(digits.parse().unwrap_or(usize::MAX))
}

#[cfg(test)]
mod tests;
