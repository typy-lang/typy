//! The nesting CPython's tokenizer refuses before it parses: brackets more than 200 deep and
//! blocks more than 99 deep, with its messages (LEX-11). Found from the tokens, before the
//! parser runs, so no nesting reaches a recursion it would make too deep.

use ruff_python_ast::token::TokenKind;
use ruff_python_parser::Mode;
use ruff_python_parser::lexer::lex;
use typy_diagnostics::{Diagnostic, RuleId};

use super::Converter;

/// The brackets CPython allows open at once, `MAXLEVEL` of its tokenizer.
pub(super) const MAX_BRACKETS: usize = 200;

/// The indentation levels CPython allows, one less than `MAXINDENT` of its tokenizer.
pub(super) const MAX_INDENTS: usize = 99;

impl Converter<'_> {
    /// Returns the error CPython gives for the first bracket or block nested past its limit.
    pub(super) fn nesting_error(&self) -> Option<Diagnostic> {
        let mut lexer = lex(self.text, Mode::Module);
        let mut brackets = 0_usize;
        let mut indents = 0_usize;
        loop {
            match lexer.next_token() {
                TokenKind::EndOfFile => return None,
                TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace => {
                    brackets += 1;
                    if brackets > MAX_BRACKETS {
                        let range = lexer.current_range();
                        return Some(
                            self.syntax_error(range, "too many nested parentheses")
                                .with_rule(RuleId::LEX_11),
                        );
                    }
                }
                TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace => {
                    brackets = brackets.saturating_sub(1);
                }
                TokenKind::Indent => {
                    indents += 1;
                    if indents > MAX_INDENTS {
                        let range = lexer.current_range();
                        return Some(
                            self.syntax_error(range, "too many levels of indentation")
                                .with_rule(RuleId::LEX_11),
                        );
                    }
                }
                TokenKind::Dedent => indents = indents.saturating_sub(1),
                _ => {}
            }
        }
    }
}
