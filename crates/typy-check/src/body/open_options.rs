//! Checks the arguments of `open` after its mode (LIB-14): `buffering`, `encoding`, `errors`
//! and `newline`, which a text file takes, and `closefd` and `opener`, which are planned.

use typy_diagnostics::RuleId;
use typy_source::syntax::{Constant, Expr};

use super::BodyChecker;
use crate::method::ParameterType;
use crate::typed::{OpenOptions, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

/// The values of `newline` CPython's `TextIOWrapper` takes besides `None`.
const NEWLINES: [&str; 4] = ["", "\n", "\r", "\r\n"];

/// Returns whether `value` is the literal `None`.
fn is_none(value: &Expr) -> bool {
    matches!(
        value,
        Expr::Constant {
            value: Constant::None,
            ..
        }
    )
}

impl BodyChecker<'_> {
    /// Returns the options of `open` from its arguments by position in CPython's order, or the
    /// error of one it refuses.
    pub(super) fn open_options(
        &mut self,
        given: &[Option<&Expr>; 8],
    ) -> Result<OpenOptions, TypedExpr> {
        if let Some(closefd) = given[6]
            && !matches!(
                closefd,
                Expr::Constant {
                    value: Constant::Bool(true),
                    ..
                }
            )
        {
            return Err(self.open_refused(
                201,
                closefd.span(),
                "the `closefd` argument of `open` is planned for TyPy 0.4 (LIB-14)".to_owned(),
                RuleId::LIB_14,
            ));
        }
        if let Some(opener) = given[7]
            && !is_none(opener)
        {
            return Err(self.open_refused(
                201,
                opener.span(),
                "the `opener` argument of `open` is planned for TyPy 1.0 (LIB-18)".to_owned(),
                RuleId::LIB_18,
            ));
        }
        let buffering = given[2]
            .filter(|value| super::expression::literal_position(value) != Some(-1))
            .map(|value| {
                let checked = self.expression(value);
                self.expect(&checked, Type::Int, "`buffering` of `open`");
                Box::new(checked)
            });
        let encoding = given[3].filter(|value| !is_none(value));
        let codec = match encoding {
            Some(value) => self.open_name(&ParameterType::Codec, value, "`encoding` of `open`")?,
            None => 0,
        };
        let handler = match given[4].filter(|value| !is_none(value)) {
            Some(value) => self.open_name(
                &ParameterType::Handler { decoding: false },
                value,
                "`errors` of `open`",
            )?,
            None => 0,
        };
        let newline = match given[5].filter(|value| !is_none(value)) {
            Some(value) => Some(Box::new(self.open_newline(value)?)),
            None => None,
        };
        Ok(OpenOptions {
            buffering,
            encoding: encoding
                .and_then(super::open_call::literal)
                .map(str::to_owned),
            codec,
            handler,
            newline,
            mode: None,
        })
    }

    /// Returns the number of the codec or error handler a string literal names.
    fn open_name(
        &mut self,
        kind: &ParameterType,
        value: &Expr,
        what: &str,
    ) -> Result<i64, TypedExpr> {
        let checked = self.codec_argument(kind, value, what);
        match &checked.kind {
            TypedExprKind::Constant(TypedConstant::Int(number)) => {
                Ok(number.parse().unwrap_or_default())
            }
            _ => Err(checked),
        }
    }

    /// Returns `newline` checked as a `str` or `None`; a literal CPython refuses is E0404 with
    /// its message, and another value is checked when the file is opened.
    fn open_newline(&mut self, value: &Expr) -> Result<TypedExpr, TypedExpr> {
        if let Some(text) = super::open_call::literal(value)
            && !NEWLINES.contains(&text)
        {
            return Err(self.open_refused(
                404,
                value.span(),
                format!(
                    "`open` raises `ValueError: illegal newline value: {text}` for this newline"
                ),
                RuleId::LIB_14,
            ));
        }
        let checked = self.expression(value);
        self.expect(
            &checked,
            Type::Optional(Box::new(Type::Str)),
            "`newline` of `open`",
        );
        Ok(checked)
    }
}
