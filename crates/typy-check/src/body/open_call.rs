//! Checks calls of `open` (LIB-04, LIB-14, ADR-0019): a `str` path and a mode written as a
//! literal, which decides that the file is a text file; `open_options` checks the rest.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Identifier, Keyword};

use super::BodyChecker;
use crate::open::{ModeRefusal, OpenAccess, open_access};
use crate::typed::{FileAttribute, TypedExpr, TypedExprKind};
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

/// Returns the text of a string literal.
pub(super) fn literal(expression: &Expr) -> Option<&str> {
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
        let options = match self.open_options(&given) {
            Ok(options) => options,
            Err(refused) => return refused,
        };
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
                options: Box::new(options),
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

    /// Returns `name`, `mode` or `closed` of a text file, or reports another name (LIB-14).
    pub(super) fn file_attribute(
        &mut self,
        file: TypedExpr,
        attribute: &Identifier,
        span: Span,
    ) -> TypedExpr {
        let Some((kind, ty)) = FileAttribute::named(&attribute.name) else {
            return self.missing_file_name(attribute, span);
        };
        Self::typed(
            TypedExprKind::FileAttribute {
                file: Box::new(file),
                attribute: kind,
            },
            ty,
            span,
        )
    }

    /// Reports a method or attribute a text file does not have, as CPython's
    /// `AttributeError` names it.
    pub(super) fn missing_file_name(&mut self, name: &Identifier, span: Span) -> TypedExpr {
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(412),
                name.span,
                format!("a TextIOWrapper has no attribute `{}` in TyPy", name.name),
            )
            .with_rule(RuleId::LIB_04),
            span,
        )
    }

    /// Reports an error of `open` and returns the expression of a call that failed.
    pub(super) fn open_refused(
        &mut self,
        code: u16,
        at: Span,
        message: String,
        rule: RuleId,
    ) -> TypedExpr {
        self.diagnostics
            .push(Diagnostic::new(DiagnosticCode::error(code), at, message).with_rule(rule));
        Self::invalid_call(at)
    }
}
