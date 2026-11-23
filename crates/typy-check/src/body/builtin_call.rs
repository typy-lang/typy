//! Checks calls of the builtins of 0.1 (LIB-01).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use super::iterable_call::takes_one_iterable;
use crate::builtin::Builtin;
use crate::typed::{PrintCall, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    #[allow(clippy::too_many_lines, reason = "one arm per builtin")]
    pub(super) fn call_builtin(
        &mut self,
        builtin: Builtin,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        if builtin == Builtin::Print {
            return self.print(arguments, keywords, span);
        }
        if builtin == Builtin::List {
            return self.call_list(arguments, keywords, None, span);
        }
        if builtin == Builtin::Bytes {
            return self.call_bytes(arguments, keywords, span);
        }
        if builtin == Builtin::Bytearray {
            return self.call_bytearray(arguments, keywords, span);
        }
        if builtin == Builtin::Tuple {
            return self.call_tuple(arguments, keywords, None, span);
        }
        if builtin == Builtin::Set {
            return self.call_set(arguments, keywords, None, span);
        }
        if builtin == Builtin::Dict {
            return self.call_dict(arguments, keywords, None, span);
        }
        if builtin == Builtin::Type {
            return self.call_type(arguments, keywords, span);
        }
        if builtin == Builtin::IsInstance {
            return self.call_isinstance(arguments, keywords, span);
        }
        if builtin == Builtin::Callable {
            return self.call_callable(arguments, keywords, span);
        }
        if builtin == Builtin::IsSubclass {
            return self.call_issubclass(arguments, keywords, span);
        }
        if builtin == Builtin::Round {
            return self.call_round(arguments, keywords, span);
        }
        if matches!(
            builtin,
            Builtin::Divmod
                | Builtin::Pow
                | Builtin::Bin
                | Builtin::Hex
                | Builtin::Oct
                | Builtin::Format
        ) {
            return self.call_number_builtin(builtin, arguments, keywords, span);
        }
        if builtin == Builtin::Iter {
            return self.call_iter(arguments, keywords, span);
        }
        if builtin == Builtin::Next {
            return self.call_next(arguments, keywords, span);
        }
        if builtin == Builtin::Open {
            return self.call_open(arguments, keywords, span);
        }
        if takes_one_iterable(builtin, arguments.len()) {
            return self.call_over_iterable(builtin, arguments, keywords, span);
        }
        let mut values: Vec<TypedExpr> = arguments
            .iter()
            .map(|argument| {
                let value = self.expression(argument);
                // `len` of an enum class is its number of members, and a member of an `IntEnum`
                // or a `StrEnum` is its value to the builtins of numbers and text (CLS-44).
                let value = if matches!(
                    builtin,
                    Builtin::Len
                        | Builtin::Range
                        | Builtin::Abs
                        | Builtin::Int
                        | Builtin::Float
                        | Builtin::Bool
                        | Builtin::Chr
                        | Builtin::Ord
                ) {
                    let value = self.enum_members(value);
                    self.enum_value(value)
                } else {
                    value
                };
                if builtin == Builtin::Len {
                    self.tuple_view(value)
                } else {
                    value
                }
            })
            .collect();
        // `len` of a `ctypes` array is its number of items (FFI-03).
        if builtin == Builtin::Len
            && keywords.is_empty()
            && let [value] = values.as_slice()
            && let Type::CData(crate::ctypes::CData::Array(..)) = value.ty
        {
            let value = values.into_iter().next().unwrap_or_else(|| unreachable!());
            return Self::typed(
                TypedExprKind::CData(crate::typed::CDataExpr::Length(Box::new(value))),
                Type::Int,
                span,
            );
        }
        // `len` of an instance calls its class's `__len__` (CLS-07).
        if builtin == Builtin::Len
            && keywords.is_empty()
            && let [value] = values.as_slice()
            && let Some(call) = self.length_call(value, span)
        {
            return call;
        }
        // `abs` of an instance calls its class's `__abs__` (CLS-07).
        if builtin == Builtin::Abs
            && keywords.is_empty()
            && let [value] = values.as_slice()
            && let Some(call) = self.operator_unary("__abs__", value, span)
        {
            return call;
        }
        // `min(a, b, key=f)` and `max` compare the values by their keys, and `key=None` is no
        // key (FUN-05).
        let keywords = match keywords {
            [keyword]
                if matches!(builtin, Builtin::Min | Builtin::Max)
                    && values.len() >= 2
                    && keyword.name.as_ref().is_some_and(|name| name.name == "key") =>
            {
                if let Expr::Constant {
                    value: typy_source::syntax::Constant::None,
                    ..
                } = keyword.value
                {
                    &[]
                } else if values.iter().any(|value| value.ty.has_error()) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                } else {
                    return self.keyed_values(
                        builtin == Builtin::Max,
                        values,
                        &keyword.value,
                        span,
                    );
                }
            }
            other => other,
        };
        // A source of a loop read as a value of its own is planned, whatever its keywords.
        if matches!(
            builtin,
            Builtin::Enumerate | Builtin::Zip | Builtin::Reversed
        ) {
            return self.loop_only(builtin, span);
        }
        if let Some(keyword) = keywords.first() {
            // `min(a, b, key=f)` and `max` were taken above; any other keyword is CPython's
            // `TypeError`, with its message.
            let name = builtin.name();
            let message = match (
                builtin,
                keyword.name.as_ref().map(|name| name.name.as_str()),
            ) {
                (Builtin::Min | Builtin::Max, Some("default")) => format!(
                    "Cannot specify a default for {name}() with multiple positional arguments"
                ),
                (Builtin::Min | Builtin::Max, Some("key")) => {
                    format!("{name} expected at least 1 argument, got {}", values.len())
                }
                (Builtin::Min | Builtin::Max, Some(other)) => {
                    format!("{name}() got an unexpected keyword argument '{other}'")
                }
                _ => format!("{name}() takes no keyword arguments"),
            };
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    format!("{message}: CPython raises `TypeError`"),
                )
                .with_rule(RuleId::LIB_01),
                span,
            );
        }
        if values.iter().any(|value| value.ty == Type::Error) {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        }
        let types: Vec<Type> = values.iter().map(|value| value.ty.clone()).collect();
        let result = match builtin {
            Builtin::Print
            | Builtin::Bytes
            | Builtin::Bytearray
            | Builtin::List
            | Builtin::Tuple
            | Builtin::Set
            | Builtin::Dict
            | Builtin::Round
            | Builtin::Divmod
            | Builtin::Pow
            | Builtin::Bin
            | Builtin::Hex
            | Builtin::Oct
            | Builtin::Format
            | Builtin::Iter
            | Builtin::Next
            | Builtin::Open
            | Builtin::Type
            | Builtin::IsInstance
            | Builtin::IsSubclass
            | Builtin::Callable
            | Builtin::Sorted
            | Builtin::Sum
            | Builtin::Any
            | Builtin::All
            | Builtin::Enumerate
            | Builtin::Zip
            | Builtin::Reversed => {
                unreachable!(
                    "print, round, type, the sources of a loop and the builtins of one iterable \
                     are checked above"
                )
            }
            // `input(prompt)` writes `str(prompt)` first, so the prompt may be any value.
            Builtin::Input => {
                if !self.arity(builtin, values.len(), 0..=1, span) {
                    return Self::invalid_call(span);
                }
                return Self::typed(
                    TypedExprKind::Input(values.into_iter().next().map(Box::new)),
                    Type::Str,
                    span,
                );
            }
            Builtin::Range => {
                if !self.arity(builtin, values.len(), 1..=3, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                for value in &values {
                    self.expect(value, Type::Int, "an argument of `range`");
                }
                Type::Range
            }
            Builtin::Slice => {
                if !self.arity(builtin, values.len(), 1..=3, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                // TyPy holds a slice's bounds as `int | None`, which every sequence takes.
                for value in &values {
                    self.expect(value, Type::optional(Type::Int), "an argument of `slice`");
                }
                Type::Slice
            }
            Builtin::Len => {
                if !self.arity(builtin, values.len(), 1..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                if !matches!(
                    values[0].ty,
                    Type::Tuple(_)
                        | Type::Range
                        | Type::List(_)
                        | Type::TupleOf(_)
                        | Type::Set(_)
                        | Type::Dict(..)
                        | Type::DictView(..)
                        | Type::Bytes
                        | Type::Bytearray
                ) {
                    self.expect(&values[0], Type::Str, "the argument of `len`");
                }
                Type::Int
            }
            Builtin::Abs => {
                if !self.arity(builtin, values.len(), 1..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                match &types[0] {
                    Type::Int | Type::Float => types[0].clone(),
                    // `abs` of a `bool` is the `int` it is (DAT-19).
                    Type::Bool => {
                        let value = values.remove(0);
                        values.insert(0, Self::int_of_bool(value));
                        Type::Int
                    }
                    other => self.operator_error("abs", other, other, values[0].span),
                }
            }
            Builtin::Min | Builtin::Max => {
                if !self.arity(builtin, values.len(), 2..=usize::MAX, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                let first = types[0].clone();
                if let Some(other) = values.iter().find(|value| value.ty != first) {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(404),
                            other.span,
                            format!(
                                "the arguments of `{}` must have one type, because CPython returns one of them unchanged; they are {first} and {}",
                                builtin.name(),
                                other.ty
                            ),
                        )
                        .with_rule(RuleId::TYP_09),
                        span,
                    );
                }
                if !matches!(first, Type::Int | Type::Float | Type::Str) {
                    self.comparison_error(&first, &first, span);
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                first
            }
            Builtin::Int | Builtin::Float => {
                let target = if builtin == Builtin::Int {
                    Type::Int
                } else {
                    Type::Float
                };
                if !self.arity(
                    builtin,
                    values.len(),
                    0..=usize::from(builtin == Builtin::Int) + 1,
                    span,
                ) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                if let Some(value) = values.first() {
                    if !matches!(value.ty, Type::Int | Type::Float | Type::Str | Type::Bool) {
                        self.expect(
                            value,
                            target.clone(),
                            &format!("the argument of `{}`", builtin.name()),
                        );
                    }
                    if values.len() == 2 {
                        self.expect(
                            &values[0],
                            Type::Str,
                            "the first argument of `int` with a base",
                        );
                        self.expect(&values[1], Type::Int, "the base");
                    }
                }
                target
            }
            Builtin::Str | Builtin::Repr => {
                let allowed = if builtin == Builtin::Str {
                    0..=1
                } else {
                    1..=1
                };
                if let Some(value) = values.first() {
                    self.check_typing_text(value);
                }
                if !self.arity(builtin, values.len(), allowed, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                Type::Str
            }
            Builtin::Bool => {
                if !self.arity(builtin, values.len(), 0..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                if let Some(value) = values.first() {
                    self.check_truth(value);
                }
                Type::Bool
            }
            Builtin::Ord => {
                if !self.arity(builtin, values.len(), 1..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                self.expect(&values[0], Type::Str, "the argument of `ord`");
                Type::Int
            }
            Builtin::Chr => {
                if !self.arity(builtin, values.len(), 1..=1, span) {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                self.expect(&values[0], Type::Int, "the argument of `chr`");
                // `chr` gives any code point, a surrogate among them (LEX-10).
                self.program.makes_surrogates = true;
                Type::Str
            }
        };
        Self::typed(
            TypedExprKind::Builtin {
                builtin,
                arguments: values,
            },
            result,
            span,
        )
    }

    pub(super) fn print(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let mut each = Vec::new();
        let mut values = Vec::new();
        for argument in arguments {
            // `*iterable` gives each of its values to `print`, read before the keywords are
            // (EXP-17, ADR-0073).
            if let Expr::Starred { value, .. } = argument {
                each.push(values.len());
                values.push(self.unpacked_list(value));
                continue;
            }
            let value = self
                .written_part(argument, false)
                .unwrap_or_else(|| self.expression(argument));
            self.check_typing_text(&value);
            values.push(value);
        }
        if !each.is_empty() {
            self.unpacking_calls.push(span);
        }
        let mut sep = None;
        let mut end = None;
        let mut file = None;
        let mut flush = None;
        let mut options: Vec<TypedExpr> = Vec::new();
        for keyword in keywords {
            let value = self.expression(&keyword.value);
            let Some(name) = &keyword.name else {
                // `**dict` binds its keys to the keywords when the call runs (EXP-22).
                self.unpacking_calls.push(span);
                if let Some(error) = self.print_options(&value, options.first()) {
                    return error;
                }
                options.push(value);
                continue;
            };
            match name.name.as_str() {
                "sep" => {
                    self.expect(&value, Type::Str, "`sep`");
                    sep = Some(value);
                }
                "end" => {
                    self.expect(&value, Type::Str, "`end`");
                    end = Some(value);
                }
                // `file=None` is standard output, as no `file` is.
                "file" if value.ty == Type::NoneType => {}
                "file" => {
                    // A text file opened for a file descriptor writes as any other.
                    if value.ty != Type::DescriptorFile(None) {
                        self.expect(&value, Type::TextFile, "`file`");
                    }
                    file = Some(value);
                }
                "flush" => {
                    self.expect(&value, Type::Bool, "`flush`");
                    flush = Some(value);
                }
                other => {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(404),
                            name.span,
                            format!("`print` has no parameter named `{other}`"),
                        )
                        .with_rule(RuleId::LIB_01),
                        span,
                    );
                }
            }
        }
        Self::typed(
            TypedExprKind::Print(Box::new(PrintCall {
                values,
                each,
                sep,
                end,
                file,
                flush,
                options,
            })),
            Type::NoneType,
            span,
        )
    }

    /// Returns the error for `**value` in a call of `print` that is no `dict[str, V]`, or
    /// whose `V` is not that of the dict of an earlier `**` (EXP-22).
    fn print_options(&mut self, value: &TypedExpr, first: Option<&TypedExpr>) -> Option<TypedExpr> {
        let span = value.span;
        let message = match (&value.ty, first.map(|first| &first.ty)) {
            (Type::Error, _) => return Some(value.clone()),
            (Type::Dict(key, _), None) if **key == Type::Str => return None,
            (Type::Dict(key, own), Some(Type::Dict(_, earlier)))
                if **key == Type::Str && own == earlier =>
            {
                return None;
            }
            (Type::Dict(key, own), Some(Type::Dict(_, earlier))) if **key == Type::Str => format!(
                "the dicts `**` gives a call of `print` hold values of one type; this holds \
                 {own}, and the first holds {earlier}"
            ),
            (other, _) => format!(
                "`**` in a call of `print` takes a dict of `str` keys, and this is {}",
                other.with_article()
            ),
        };
        Some(self.error(
            Diagnostic::new(DiagnosticCode::error(404), span, message).with_rule(RuleId::EXP_22),
            span,
        ))
    }

    pub(super) fn arity(
        &mut self,
        builtin: Builtin,
        given: usize,
        allowed: std::ops::RangeInclusive<usize>,
        span: Span,
    ) -> bool {
        if allowed.contains(&given) {
            return true;
        }
        let expected = if allowed.start() == allowed.end() {
            format!("{}", allowed.start())
        } else {
            format!("{} to {}", allowed.start(), allowed.end())
        };
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                span,
                format!(
                    "`{}` takes {expected} arguments, and {given} are given",
                    builtin.name()
                ),
            )
            .with_rule(RuleId::LIB_01),
        );
        false
    }

    /// Returns E0201 for `enumerate`, `zip` or `reversed` called or read as a value outside
    /// a `for` loop, a comprehension or an iterable argument, where TyPy makes no object of
    /// it, which is planned for after 1.0.
    pub(super) fn loop_only(&mut self, builtin: Builtin, span: Span) -> TypedExpr {
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!(
                    "`{}` outside a `for` loop, a comprehension or an iterable argument is \
                     planned for after TyPy 1.0",
                    builtin.name()
                ),
            )
            .with_rule(RuleId::CST_09),
            span,
        )
    }
}
