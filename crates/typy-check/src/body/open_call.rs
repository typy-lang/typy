//! Checks calls of `open` (LIB-04, LIB-14, ADR-0019): a `str` path, a mode written as a
//! literal, which decides that the file is a text file, and UTF-8 as its encoding.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Keyword};

use super::BodyChecker;
use crate::open::{ModeRefusal, OpenAccess, is_utf8, open_access};
use crate::typed::{TypedExpr, TypedExprKind};
use crate::types::Type;

/// The parameters of `open`, in CPython's order.
const PARAMETERS: [&str; 8] = [
    "file",
    "mode",
    "buffering",
    "encoding",
    "errors",
    "newline",
    "closefd",
    "opener",
];

/// Returns whether `value` is the default of parameter `index` of `open`, which an argument
/// may spell out: `buffering=-1`, `errors=None` or `"strict"`, `newline=None`, `closefd=True`
/// and `opener=None`.
fn is_default(index: usize, value: &Expr) -> bool {
    let negative_one = super::expression::literal_position(value) == Some(-1);
    let none = matches!(
        value,
        Expr::Constant {
            value: Constant::None,
            ..
        }
    );
    match PARAMETERS[index] {
        "buffering" => negative_one,
        "errors" => none || literal(value) == Some("strict"),
        "closefd" => matches!(
            value,
            Expr::Constant {
                value: Constant::Bool(true),
                ..
            }
        ),
        _ => none,
    }
}

/// Returns the text of a string literal.
fn literal(expression: &Expr) -> Option<&str> {
    match expression {
        Expr::Constant {
            value: Constant::Str(text),
            ..
        } => Some(text),
        _ => None,
    }
}

impl BodyChecker<'_> {
    /// Checks `open(file, mode, encoding=encoding)`, which gives a `TextIOWrapper`.
    pub(super) fn call_open(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let mut given: [Option<&Expr>; 8] = [None; 8];
        for (index, argument) in arguments.iter().enumerate() {
            if index >= PARAMETERS.len() {
                return self.open_refused(
                    404,
                    argument.span(),
                    format!(
                        "`open` takes at most 8 arguments, and {} are given",
                        arguments.len()
                    ),
                    RuleId::LIB_04,
                );
            }
            given[index] = Some(argument);
        }
        // A call with `**` arguments is refused before a builtin is checked.
        let named = keywords
            .iter()
            .filter_map(|keyword| keyword.name.as_ref().map(|name| (keyword, name)));
        for (keyword, name) in named {
            let Some(index) = PARAMETERS
                .iter()
                .position(|parameter| *parameter == name.name)
            else {
                return self.open_refused(
                    404,
                    name.span,
                    format!("`open` has no parameter named `{}`", name.name),
                    RuleId::LIB_04,
                );
            };
            if given[index].is_some() {
                return self.open_refused(
                    404,
                    name.span,
                    format!("`open` is given `{}` twice", name.name),
                    RuleId::LIB_04,
                );
            }
            given[index] = Some(&keyword.value);
        }
        let Some(file) = given[0] else {
            return self.open_refused(
                404,
                span,
                "`open` needs the file to open".to_owned(),
                RuleId::LIB_04,
            );
        };
        let path = self.expression(file);
        self.expect(&path, Type::Str, "the file of `open`");
        if let Some((index, value)) = given
            .iter()
            .enumerate()
            .skip(2)
            .filter_map(|(index, value)| value.map(|value| (index, value)))
            .find(|(index, value)| *index != 3 && !is_default(*index, value))
        {
            return self.open_refused(
                201,
                value.span(),
                format!(
                    "the `{}` argument of `open` is planned for TyPy 0.4 (LIB-14)",
                    PARAMETERS[index]
                ),
                RuleId::LIB_14,
            );
        }
        if let Some(encoding) = given[3]
            && let Some(refused) = self.open_encoding(encoding)
        {
            return refused;
        }
        let (mode, access) = match given[1] {
            None => ("r".to_owned(), OpenAccess::Read),
            Some(mode) => match self.open_mode(mode) {
                Ok(found) => found,
                Err(refused) => return refused,
            },
        };
        if path.ty.has_error() {
            return Self::invalid_call(span);
        }
        Self::typed(
            TypedExprKind::Open {
                path: Box::new(path),
                mode,
                access,
            },
            Type::TextFile,
            span,
        )
    }

    /// Returns the mode of `open` and its access, or the error of a mode it refuses.
    fn open_mode(&mut self, mode: &Expr) -> Result<(String, OpenAccess), TypedExpr> {
        let Some(text) = literal(mode) else {
            let value = self.expression(mode);
            if value.ty.has_error() {
                return Err(Self::invalid_call(mode.span()));
            }
            return Err(self.open_refused(
                201,
                mode.span(),
                "a mode of `open` that is not a string literal is planned for TyPy 0.4 (LIB-14)"
                    .to_owned(),
                RuleId::LIB_14,
            ));
        };
        match open_access(text) {
            Ok(access) => Ok((text.to_owned(), access)),
            Err(ModeRefusal::Fails(message)) => Err(self.open_refused(
                404,
                mode.span(),
                format!("`open` raises `ValueError: {message}` for this mode"),
                RuleId::LIB_04,
            )),
            Err(ModeRefusal::Planned(what)) => Err(self.open_refused(
                201,
                mode.span(),
                format!("{what} of `open` is planned for TyPy 0.4 (LIB-14)"),
                RuleId::LIB_14,
            )),
        }
    }

    /// Returns the error of an encoding that is neither `None` nor a name of UTF-8.
    fn open_encoding(&mut self, encoding: &Expr) -> Option<TypedExpr> {
        match encoding {
            Expr::Constant {
                value: Constant::None,
                ..
            } => None,
            _ if literal(encoding).is_some_and(is_utf8) => None,
            _ => Some(
                self.open_refused(
                    201,
                    encoding.span(),
                    "TyPy 0.3 opens files in UTF-8 only, written as `\"utf-8\"` or `None`; other \
                 encodings are planned for TyPy 0.4 (LIB-14)"
                        .to_owned(),
                    RuleId::LIB_14,
                ),
            ),
        }
    }

    /// Reports a method or attribute a text file does not have in 0.3: E0201 when it is
    /// planned for 0.4, E0412 otherwise.
    pub(super) fn missing_file_name(&mut self, name: &str, at: Span, span: Span) -> TypedExpr {
        let (message, planned) = crate::open::missing_file_name(name);
        let (code, rule) = if planned {
            (201, RuleId::LIB_14)
        } else {
            (412, RuleId::LIB_04)
        };
        self.diagnostics
            .push(Diagnostic::new(DiagnosticCode::error(code), at, message).with_rule(rule));
        Self::invalid_call(span)
    }

    /// Reports an error of `open` and returns the expression of a call that failed.
    fn open_refused(&mut self, code: u16, at: Span, message: String, rule: RuleId) -> TypedExpr {
        self.diagnostics
            .push(Diagnostic::new(DiagnosticCode::error(code), at, message).with_rule(rule));
        Self::invalid_call(at)
    }
}
