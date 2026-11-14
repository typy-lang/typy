//! Checks calls of `open` (LIB-04, LIB-14, ADR-0019): a `str` path and a mode written as a
//! literal, which decides that the file is a text file; `open_options` checks the rest.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Identifier, Keyword};

use super::BodyChecker;
use crate::open::{ModeRefusal, OpenAccess, OpenMode, open_access};
use crate::typed::{FileAttribute, TypedExpr, TypedExprKind};
use crate::types::{BinaryKind, Type};

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
        // CPython reads the mode first, then refuses what a binary mode does not take.
        let (written, mode) = match given[1] {
            None => (
                "r".to_owned(),
                OpenMode {
                    access: OpenAccess::Read,
                    binary: false,
                },
            ),
            Some(mode) => match self.open_mode(mode) {
                Ok(found) => found,
                Err(refused) => return refused,
            },
        };
        let ty = if mode.binary {
            match self.binary_file(&given, mode) {
                Ok(kind) => Type::BinaryFile(kind),
                Err(refused) => return refused,
            }
        } else {
            Type::TextFile
        };
        let mut options = match self.open_options(&given) {
            Ok(options) => options,
            Err(refused) => return refused,
        };
        let shown = if mode.binary {
            // The buffer's size changes nothing a program sees, so a binary file has none.
            options.buffering = None;
            mode.binary_mode().to_owned()
        } else {
            written
        };
        if path.ty.has_error() {
            return Self::invalid_call(span);
        }
        Self::typed(
            TypedExprKind::Open {
                path: Box::new(path),
                mode: shown,
                access: mode.access,
                options: Box::new(options),
            },
            ty,
            span,
        )
    }

    /// Returns the class of a binary file, or the error of an argument a binary mode does not
    /// take: `encoding`, `errors` and `newline` raise CPython's `ValueError`; `buffering=0`
    /// gives a `FileIO`, and `buffering=1` makes CPython warn with the program's own file and
    /// line, so it is refused; a `buffering` that is not a literal decides the class only
    /// when the program runs, which is planned for 1.0 (LIB-19).
    fn binary_file(
        &mut self,
        given: &[Option<&Expr>; 8],
        mode: OpenMode,
    ) -> Result<BinaryKind, TypedExpr> {
        for (index, what) in [(3, "an encoding"), (4, "an errors"), (5, "a newline")] {
            if let Some(value) = given[index]
                && !matches!(
                    value,
                    Expr::Constant {
                        value: Constant::None,
                        ..
                    }
                )
            {
                return Err(self.open_refused(
                    404,
                    value.span(),
                    format!(
                        "`open` raises `ValueError: binary mode doesn't take {what} argument` \
                         for this mode"
                    ),
                    RuleId::LIB_14,
                ));
            }
        }
        let buffered = match mode.access {
            OpenAccess::Read => BinaryKind::Reader,
            OpenAccess::Write | OpenAccess::Append | OpenAccess::Create => BinaryKind::Writer,
            _ => BinaryKind::Random,
        };
        let Some(buffering) = given[2] else {
            return Ok(buffered);
        };
        match super::expression::literal_position(buffering) {
            Some(0) => Ok(BinaryKind::Raw),
            Some(1) => Err(self.open_refused(
                404,
                buffering.span(),
                "CPython warns that `buffering=1` does not buffer lines in a binary mode, \
                 naming the program's file and line; leave it out"
                    .to_owned(),
                RuleId::LIB_14,
            )),
            Some(_) => Ok(buffered),
            None => {
                let value = self.expression(buffering);
                if value.ty.has_error() {
                    return Err(Self::invalid_call(buffering.span()));
                }
                Err(self.open_refused(
                    201,
                    buffering.span(),
                    "a `buffering` that is not a literal decides the class of a binary file \
                     when the program runs, which is planned for TyPy 1.0 (LIB-19)"
                        .to_owned(),
                    RuleId::LIB_19,
                ))
            }
        }
    }

    /// Returns the mode of `open` as written and what it gives, or the error of a mode it
    /// refuses.
    fn open_mode(&mut self, mode: &Expr) -> Result<(String, OpenMode), TypedExpr> {
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
            Ok(found) => Ok((text.to_owned(), found)),
            Err(ModeRefusal(message)) => Err(self.open_refused(
                404,
                mode.span(),
                format!("`open` raises `ValueError: {message}` for this mode"),
                RuleId::LIB_04,
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
            return self.missing_file_name(&file.ty.clone(), attribute, span);
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
    pub(super) fn missing_file_name(
        &mut self,
        file: &Type,
        name: &Identifier,
        span: Span,
    ) -> TypedExpr {
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(412),
                name.span,
                format!(
                    "{} has no attribute `{}` in TyPy",
                    file.with_article(),
                    name.name
                ),
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
