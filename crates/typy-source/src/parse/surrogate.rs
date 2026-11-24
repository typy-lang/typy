//! Keeps the surrogates of `str` literals (LEX-10). The parser gives a literal's value as
//! UTF-8 text, which cannot hold U+D800 to U+DFFF, so it writes U+FFFD for each `\u` or `\U`
//! escape of one. In that text the n-th U+FFFD comes from the n-th item of the source that
//! gives one: the character itself, its escapes and the surrogate escapes. Reading the source
//! in order puts each surrogate back where CPython keeps it.

use ruff_text_size::TextRange;

use super::Converter;

/// The code point the parser writes for an escape of a surrogate.
const REPLACEMENT: char = '\u{fffd}';

impl Converter<'_> {
    /// Returns the code points of the literal text `value`, whose source without prefix and
    /// quotes is `content`, with each escaped surrogate in place; `None` when it has none.
    pub(super) fn surrogate_points(&self, value: &str, content: TextRange) -> Option<Vec<u32>> {
        let sources = replacement_sources(&self.text[content]);
        if !sources.iter().any(Option::is_some) {
            return None;
        }
        let mut sources = sources.into_iter();
        let points = value
            .chars()
            .map(|character| match (character, character == REPLACEMENT) {
                (_, true) => sources.next().flatten().unwrap_or(u32::from(REPLACEMENT)),
                (other, false) => u32::from(other),
            })
            .collect();
        Some(points)
    }
}

/// Returns, in source order, each item of the literal text `text` that gives U+FFFD in the
/// parser's value: the surrogate an escape names, or `None` for U+FFFD itself.
fn replacement_sources(text: &str) -> Vec<Option<u32>> {
    let mut sources = Vec::new();
    let mut at = 0;
    while let Some(character) = text[at..].chars().next() {
        at += character.len_utf8();
        if character == REPLACEMENT {
            sources.push(None);
            continue;
        }
        if character != '\\' {
            continue;
        }
        // A backslash escapes the next character, so `\\u` is no escape of `u`.
        let Some(escaped) = text[at..].chars().next() else {
            break;
        };
        at += escaped.len_utf8();
        let rest = &text[at..];
        match escaped {
            'u' | 'U' => {
                let width = if escaped == 'u' { 4 } else { 8 };
                let Some(point) = hex_digits(rest, width) else {
                    continue;
                };
                at += width;
                if (0xd800..=0xdfff).contains(&point) {
                    sources.push(Some(point));
                } else if point == u32::from(REPLACEMENT) {
                    sources.push(None);
                }
            }
            'N' => {
                let Some((name, _)) = rest.strip_prefix('{').and_then(|rest| rest.split_once('}'))
                else {
                    continue;
                };
                at += name.len() + 2;
                if name.eq_ignore_ascii_case("REPLACEMENT CHARACTER") {
                    sources.push(None);
                }
            }
            _ => {}
        }
    }
    sources
}

/// Returns the code point the first `width` characters of `text` give as hexadecimal digits.
fn hex_digits(text: &str, width: usize) -> Option<u32> {
    let digits = text.get(..width)?;
    if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(digits, 16).ok()
}

#[cfg(test)]
mod tests;
