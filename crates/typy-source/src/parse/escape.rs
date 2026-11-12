//! Finds the escape sequences CPython 3.12 to 3.15 warn about while compiling, in every
//! string, bytes and f-string token that is not raw: W0101 (DIA-08).

use ruff_python_ast::StringFlags as _;
use ruff_python_ast::token::{TokenKind, Tokens};
use ruff_text_size::{Ranged as _, TextRange, TextSize};
use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};

use super::Converter;

/// The characters a backslash may come before in a `str` literal; a `bytes` literal takes
/// the same but `N`, `u` and `U`. Digits start an octal escape, a line break a joined line.
const STR_ESCAPES: &str = "\\'\"abfnrtvxNuU01234567\n\r";
const BYTES_ESCAPES: &str = "\\'\"abfnrtvx01234567\n\r";

impl Converter<'_> {
    /// Returns a warning for each escape sequence CPython would warn about.
    pub(super) fn escape_warnings(&self, tokens: &Tokens) -> Vec<Diagnostic> {
        let mut warnings = Vec::new();
        for token in tokens {
            let Some(flags) = token.string_flags() else {
                continue;
            };
            if flags.is_raw_string() {
                continue;
            }
            let range = token.range();
            let content = match token.kind() {
                TokenKind::String => TextRange::new(
                    range.start() + flags.opener_len(),
                    range.end() - flags.closer_len(),
                ),
                TokenKind::FStringMiddle | TokenKind::TStringMiddle => range,
                _ => continue,
            };
            let allowed = if flags.is_byte_string() {
                BYTES_ESCAPES
            } else {
                STR_ESCAPES
            };
            self.scan(content, allowed, &mut warnings);
        }
        warnings
    }

    /// Adds a warning for each invalid escape in the text of `content`. The character after
    /// a backslash is read from the source, since at the end of an f-string's literal text
    /// it is the `{` of the next field, which CPython also warns about.
    fn scan(&self, content: TextRange, allowed: &str, warnings: &mut Vec<Diagnostic>) {
        let base = usize::from(content.start());
        let text = &self.text[content];
        let mut at = 0;
        while let Some(found) = text.get(at..).and_then(|rest| rest.find('\\')) {
            let backslash = at + found;
            let after = &self.text[base + backslash + 1..];
            let next = after.chars().next().unwrap_or('\\');
            let start = content.start() + offset(backslash);
            let digits: String = after
                .chars()
                .take(3)
                .take_while(|digit| digit.is_digit(8))
                .collect();
            at = backslash + 1 + next.len_utf8().max(digits.len());
            if !allowed.contains(next) {
                let span = self.span(TextRange::new(start, start + offset(1 + next.len_utf8())));
                let escape = format!("\\{next}");
                warnings.push(
                    Diagnostic::new(
                        DiagnosticCode::warning(101),
                        span,
                        format!("`{escape}` is an invalid escape sequence"),
                    )
                    .with_rule(RuleId::DIA_08)
                    .with_help("CPython keeps the backslash; write it twice, or use a raw string")
                    .with_fix(span, format!("\\{escape}")),
                );
            } else if u32::from_str_radix(&digits, 8).is_ok_and(|value| value > 0o377) {
                let end = start + offset(1 + digits.len());
                warnings.push(
                    Diagnostic::new(
                        DiagnosticCode::warning(101),
                        self.span(TextRange::new(start, end)),
                        format!("`\\{digits}` is an invalid octal escape sequence"),
                    )
                    .with_rule(RuleId::DIA_08)
                    .with_help("an octal escape goes up to `\\377`"),
                );
            }
        }
    }
}

/// Returns a byte offset as a `TextSize`; a source file is at most 16 MiB (LEX-09).
fn offset(bytes: usize) -> TextSize {
    TextSize::new(u32::try_from(bytes).unwrap_or(u32::MAX))
}
