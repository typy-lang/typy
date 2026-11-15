//! Checks calls of `open` (LIB-04, LIB-14, ADR-0019): a `str` path and a mode written as a
//! literal, which decides that the file is a text file; `open_options` checks the rest.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Identifier, Keyword};

use super::BodyChecker;
use crate::open::{ModeRefusal, OpenAccess, OpenMode, open_access};
use crate::typed::{FileAttribute, TypedConstant, TypedExpr, TypedExprKind};
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
        let given = match self.open_arguments(arguments, keywords) {
            Ok(given) => given,
            Err(refused) => return refused,
        };
        let Some(file) = given[0] else {
            return self.open_refused(
                404,
                span,
                "`open` needs the file to open".to_owned(),
                RuleId::LIB_04,
            );
        };
        let path = self.expression(file);
        // An `int` is a file descriptor; CPython 3.13 and later warn for a `bool`, so it is
        // refused (LIB-14).
        let descriptor = path.ty == Type::Int;
        if path.ty == Type::Bool {
            return self.open_refused(
                404,
                path.span,
                "CPython 3.13 and later warn that a bool is used as a file descriptor; pass an int"
                    .to_owned(),
                RuleId::LIB_14,
            );
        }
        if !descriptor {
            self.expect(&path, Type::Str, "the file of `open`");
        }
        // CPython reads the mode first, then refuses what a binary mode does not take.
        let text_read = OpenMode {
            access: OpenAccess::Read,
            binary: false,
        };
        let mut mode_value = None;
        let (written, mode) = match given[1] {
            None => ("r".to_owned(), text_read),
            Some(mode) => match self.open_mode(mode) {
                Ok(found) => found,
                Err(Ok(value)) => {
                    mode_value = Some(Box::new(value));
                    (String::new(), text_read)
                }
                Err(Err(refused)) => return refused,
            },
        };
        let binary = if mode.binary {
            match self.binary_file(&given, mode) {
                Ok(kind) => Some(kind),
                Err(refused) => return refused,
            }
        } else {
            None
        };
        // Reading and writing a text file, and a path with a surrogate, may raise a Unicode
        // error (ADR-0034).
        self.program.record_codec(true, true);
        if descriptor {
            self.program.record_descriptors();
        }
        let ty = match (descriptor, binary) {
            (true, kind) => Type::DescriptorFile(kind),
            (false, Some(kind)) => Type::BinaryFile(kind),
            (false, None) => Type::TextFile,
        };
        let mut options = match self.open_options(&given) {
            Ok(options) => options,
            Err(refused) => return refused,
        };
        options.mode = mode_value;
        if !descriptor
            && let Some(closefd) = &options.closefd
            && matches!(
                closefd.kind,
                TypedExprKind::Constant(TypedConstant::Bool(false))
            )
        {
            return self.open_refused(
                404,
                closefd.span,
                "`open` raises `ValueError: Cannot use closefd=False with file name` for a path"
                    .to_owned(),
                RuleId::LIB_14,
            );
        }
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

    /// Returns the arguments of `open` by position in CPython's order, from the call's
    /// positional and keyword arguments, or the error of one `open` does not take.
    fn open_arguments<'call>(
        &mut self,
        arguments: &'call [Expr],
        keywords: &'call [Keyword],
    ) -> Result<[Option<&'call Expr>; 8], TypedExpr> {
        let mut given: [Option<&Expr>; 8] = [None; 8];
        for (index, argument) in arguments.iter().enumerate() {
            if index >= PARAMETERS.len() {
                return Err(self.open_refused(
                    404,
                    argument.span(),
                    format!(
                        "`open` takes at most 8 arguments, and {} are given",
                        arguments.len()
                    ),
                    RuleId::LIB_04,
                ));
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
                return Err(self.open_refused(
                    404,
                    name.span,
                    format!("`open` has no parameter named `{}`", name.name),
                    RuleId::LIB_04,
                ));
            };
            if given[index].is_some() {
                return Err(self.open_refused(
                    404,
                    name.span,
                    format!("`open` is given `{}` twice", name.name),
                    RuleId::LIB_04,
                ));
            }
            given[index] = Some(&keyword.value);
        }
        Ok(given)
    }

    /// Returns the mode of `open` as written and what it gives, or the error of a mode it
    /// refuses. A mode that is not a literal is a `str` the runtime reads as a text mode
    /// (ADR-0031); it is returned as `Err(Ok(value))`.
    fn open_mode(
        &mut self,
        mode: &Expr,
    ) -> Result<(String, OpenMode), Result<TypedExpr, TypedExpr>> {
        let Some(text) = literal(mode) else {
            let value = self.expression(mode);
            self.expect(&value, Type::Str, "the mode of `open`");
            if value.ty.has_error() {
                return Err(Err(Self::invalid_call(mode.span())));
            }
            return Err(Ok(value));
        };
        match open_access(text) {
            Ok(found) => Ok((text.to_owned(), found)),
            Err(ModeRefusal(message)) => Err(Err(self.open_refused(
                404,
                mode.span(),
                format!("`open` raises `ValueError: {message}` for this mode"),
                RuleId::LIB_04,
            ))),
        }
    }

    /// Returns `name`, `mode` or `closed` of a text file, or reports another name (LIB-14).
    pub(super) fn file_attribute(
        &mut self,
        file: TypedExpr,
        attribute: &Identifier,
        span: Span,
    ) -> TypedExpr {
        let Some((mut kind, mut ty)) = FileAttribute::named(&attribute.name) else {
            return self.missing_file_name(&file.ty.clone(), attribute, span);
        };
        if kind == FileAttribute::Name && matches!(file.ty, Type::DescriptorFile(_)) {
            (kind, ty) = (FileAttribute::Descriptor, Type::Int);
        }
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
