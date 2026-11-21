//! The types of operators, with Python's results (EXP-02 to EXP-09).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, CompareOperator, UnaryOperator};

use super::BodyChecker;
use super::view_operator::is_set_view;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns true when `left ** right` with a float exponent cannot be complex in CPython: the exponent is an integral literal, or the base is a non-negative literal (EXP-04).
    fn float_power_is_real(left: &TypedExpr, right: &TypedExpr) -> bool {
        let integral_exponent = matches!(&right.kind, TypedExprKind::Constant(TypedConstant::Float(value)) if value.fract() == 0.0);
        // A literal is never negative: `-1.5` is the negation of the literal `1.5`, which is an
        // operation, so a constant base is a non-negative literal.
        let literal_base = matches!(
            &left.kind,
            TypedExprKind::Constant(TypedConstant::Int(_) | TypedConstant::Float(_))
        );
        integral_exponent || literal_base
    }

    /// Returns the type of a binary operation, with Python's results (EXP-02 to EXP-05).
    pub(super) fn binary_type(
        &mut self,
        operator: BinaryOperator,
        left: &TypedExpr,
        right: &TypedExpr,
        span: Span,
    ) -> Type {
        use BinaryOperator as Op;
        use Type::{Bool, Error, Float, Int, Str};
        let (l, r) = (left.ty.clone(), right.ty.clone());
        if l == Error || r == Error {
            return Error;
        }
        let symbol = binary_symbol(operator);
        if matches!(operator, Op::BitOr | Op::BitAnd | Op::Sub | Op::BitXor)
            && (is_set_view(&l) || is_set_view(&r))
        {
            return self.view_operation(symbol, &l, &r, span);
        }
        if let (Op::Add, Type::Tuple(first), Type::Tuple(second)) = (operator, &l, &r) {
            return Type::Tuple(first.iter().chain(second).cloned().collect());
        }
        match operator {
            Op::BitAnd | Op::BitOr | Op::BitXor if l == Bool && r == Bool => Bool,
            Op::Add if l == Str && r == Str => Str,
            Op::Add if l == Type::Bytes && r == Type::Bytes => Type::Bytes,
            Op::Mult if (l == Type::Bytes && r == Int) || (l == Int && r == Type::Bytes) => {
                Type::Bytes
            }
            // A list joins only a list of its own type, because the runtime stores each item by
            // the kind of the item type (DAT-07).
            Op::Add if l.sequence_item().is_some() && l == r => l,
            // The runtime combines two sets by the item type of the first (DAT-10).
            Op::BitOr | Op::BitAnd | Op::Sub | Op::BitXor
                if matches!(l, Type::Set(_)) && l == r =>
            {
                l
            }
            // `|` merges two dicts of one type into a new one (DAT-09).
            Op::BitOr if matches!(l, Type::Dict(..)) && l == r => l,
            Op::Mult if l.sequence_item().is_some() && r == Int => l,
            Op::Mult if l == Int && r.sequence_item().is_some() => r,
            Op::Mult if (l == Str && r == Int) || (l == Int && r == Str) => Str,
            Op::Add | Op::Sub | Op::Mult | Op::FloorDiv | Op::Mod
                if l.is_numeric() && r.is_numeric() =>
            {
                if l == Int && r == Int {
                    Int
                } else {
                    Float
                }
            }
            Op::Div if l.is_numeric() && r.is_numeric() => Float,
            Op::Pow if l.is_numeric() && r.is_numeric() => {
                if l == Int && r == Int {
                    if !matches!(&right.kind, TypedExprKind::Constant(TypedConstant::Int(_))) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                DiagnosticCode::error(302),
                                right.span,
                                "the exponent of `int ** int` must be a non-negative integer literal, because CPython returns a float for a negative exponent",
                            )
                            .with_rule(RuleId::EXP_04)
                            .with_help("write the base as a float, such as `2.0 ** n`"),
                        );
                        return Error;
                    }
                    Int
                } else if r == Float && !Self::float_power_is_real(left, right) {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(302),
                            right.span,
                            "a float exponent needs an integral literal or a non-negative literal base, because CPython returns a complex number for a negative base",
                        )
                        .with_rule(RuleId::EXP_04),
                    );
                    Error
                } else {
                    Float
                }
            }
            Op::LShift | Op::RShift | Op::BitAnd | Op::BitOr | Op::BitXor
                if l == Int && r == Int =>
            {
                Int
            }
            _ => self.operator_error(symbol, &l, &r, span),
        }
    }

    pub(super) fn unary_type(
        &mut self,
        operator: UnaryOperator,
        operand: &TypedExpr,
        span: Span,
    ) -> Type {
        let ty = operand.ty.clone();
        match operator {
            _ if ty == Type::Error => Type::Error,
            UnaryOperator::Not => {
                self.check_truth(operand);
                Type::Bool
            }
            UnaryOperator::Invert if ty == Type::Bool => self.bool_inversion(span),
            UnaryOperator::USub | UnaryOperator::UAdd if ty.is_numeric() => ty,
            UnaryOperator::Invert if ty == Type::Int => Type::Int,
            _ => {
                let symbol = match operator {
                    UnaryOperator::USub => "-",
                    UnaryOperator::UAdd => "+",
                    UnaryOperator::Invert | UnaryOperator::Not => "~",
                };
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(401),
                        span,
                        format!("unary `{symbol}` is not defined for {ty}"),
                    )
                    .with_rule(RuleId::EXP_02),
                );
                Type::Error
            }
        }
    }

    /// Checks one link of a comparison chain (EXP-06, EXP-08).
    pub(super) fn compare_types(
        &mut self,
        operator: CompareOperator,
        left: &TypedExpr,
        right: &TypedExpr,
    ) {
        let (l, r) = (left.ty.clone(), right.ty.clone());
        if l == Type::Error || r == Type::Error {
            return;
        }
        let span = right.span;
        let compares = !matches!(
            operator,
            CompareOperator::In
                | CompareOperator::NotIn
                | CompareOperator::Is
                | CompareOperator::IsNot
        );
        if compares && (matches!(l, Type::DictView(..)) || matches!(r, Type::DictView(..))) {
            let equality = matches!(operator, CompareOperator::Eq | CompareOperator::NotEq);
            if !Self::view_comparison(&l, &r, equality) {
                self.comparison_error(&l, &r, span);
            }
            return;
        }
        match operator {
            CompareOperator::Is | CompareOperator::IsNot => {
                self.identity_types((left, right), span);
            }
            CompareOperator::Eq | CompareOperator::NotEq => {
                if !l.can_test_equality(&r) {
                    self.comparison_error(&l, &r, span);
                } else if let Some(class) = [&l, &r]
                    .into_iter()
                    .find_map(|side| self.exception_with_equality(side))
                {
                    // CPython calls the class's own method, which TyPy does not call for an
                    // exception yet, and comparing by identity would answer otherwise.
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(201),
                            span,
                            format!(
                                "`==` and `!=` of an exception whose class, `{class}`, defines \
                                 `__eq__` or `__ne__` are planned for TyPy 1.0 (EXC-20)"
                            ),
                        )
                        .with_rule(RuleId::EXC_20),
                    );
                }
            }
            CompareOperator::Lt
            | CompareOperator::LtE
            | CompareOperator::Gt
            | CompareOperator::GtE => {
                if !l.can_order(&r) {
                    self.comparison_error(&l, &r, span);
                }
            }
            // `x in xs` tests each item with `==`, so `x` compares for equality with the items
            // (EXP-07).
            CompareOperator::In | CompareOperator::NotIn => match &r {
                // The runtime compares the value with each item by the item's kind, so the value
                // has the item type.
                Type::List(item) | Type::TupleOf(item) | Type::Set(item) | Type::Dict(item, _)
                    if l.is_assignable_to(item) => {}
                // A view finds a key, a value, or a `(key, value)` pair.
                Type::DictView(view, key, value) if l.is_assignable_to(&view.item(key, value)) => {}
                // A substring test: `in` on a `str` needs a `str` on its left.
                Type::Str if l == Type::Str => {}
                // `in` on a range tests an `int` without stepping (DAT-17).
                Type::Range if l == Type::Int => {}
                // `in` on `bytes` looks for a byte, an `int`, or for `bytes` within them.
                Type::Bytes if matches!(l, Type::Int | Type::Bytes) => {}
                // A tuple compares the value with each item for equality, so each item has to
                // compare with it.
                Type::Tuple(items) if items.iter().all(|item| l.can_test_equality(item)) => {}
                other => self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(407),
                        span,
                        format!(
                            "`in` looks for {} in {}, which cannot hold it",
                            l.with_article(),
                            other.with_article()
                        ),
                    )
                    .with_rule(RuleId::EXP_07),
                ),
            },
        }
    }

    /// Returns the operands of a comparison with each held as an `object` when one of them
    /// is one, so the runtime compares the objects as their types compare them (TYP-23); a
    /// value `in` a collection of `object` items is held as one too.
    pub(super) fn objects_compared(
        operators: &[CompareOperator],
        operands: Vec<TypedExpr>,
    ) -> Vec<TypedExpr> {
        let as_object = |operand: TypedExpr| {
            if operand.ty.is_assignable_to(&Type::Object) {
                Self::converted(operand, Some(&Type::Object))
            } else {
                operand
            }
        };
        let identity_or_equality = operators.iter().all(|operator| {
            matches!(
                operator,
                CompareOperator::Eq
                    | CompareOperator::NotEq
                    | CompareOperator::Is
                    | CompareOperator::IsNot
            )
        });
        if identity_or_equality && operands.iter().any(|operand| operand.ty == Type::Object) {
            return operands.into_iter().map(as_object).collect();
        }
        if let ([CompareOperator::In | CompareOperator::NotIn], [value, container]) =
            (operators, operands.as_slice())
            && value.ty != Type::Object
            && matches!(&container.ty, Type::List(item) | Type::TupleOf(item) | Type::Set(item) | Type::Dict(item, _) if **item == Type::Object)
        {
            let mut operands = operands;
            let container = operands
                .pop()
                .unwrap_or_else(|| unreachable!("two operands"));
            let value = operands
                .pop()
                .unwrap_or_else(|| unreachable!("two operands"));
            return vec![as_object(value), container];
        }
        operands
    }

    /// Returns the operands of a comparison with each `bool` held as its `int` when the
    /// others are numbers, so `True == 1` and `True < 2` compare as CPython compares them
    /// (DAT-19); `is` and `in` keep them.
    pub(super) fn numbers_compared(
        operators: &[CompareOperator],
        operands: Vec<TypedExpr>,
    ) -> Vec<TypedExpr> {
        let compares = operators.iter().all(|operator| {
            !matches!(
                operator,
                CompareOperator::Is
                    | CompareOperator::IsNot
                    | CompareOperator::In
                    | CompareOperator::NotIn
            )
        });
        let mixed = operands.iter().any(|operand| operand.ty == Type::Bool)
            && operands
                .iter()
                .any(|operand| matches!(operand.ty, Type::Int | Type::Float));
        if !compares || !mixed {
            return operands;
        }
        operands.into_iter().map(Self::int_of_bool).collect()
    }

    pub(super) fn operator_error(
        &mut self,
        symbol: &str,
        left: &Type,
        right: &Type,
        span: Span,
    ) -> Type {
        let diagnostic = Diagnostic::new(
            DiagnosticCode::error(401),
            span,
            format!("`{symbol}` is not defined for {left} and {right}"),
        )
        .with_rule(RuleId::EXP_02);
        self.diagnostics
            .push(with_narrowing_help(diagnostic, left, right));
        Type::Error
    }

    /// Reports `~` of a `bool`, which CPython 3.12 and later deprecate with a warning, so TyPy
    /// refuses it (DAT-19, ADR-0098).
    pub(super) fn bool_inversion(&mut self, span: Span) -> Type {
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(406),
                span,
                "`~` of a bool is deprecated since CPython 3.12, which warns about it, and \
                 removed in 3.16, so TyPy refuses it",
            )
            .with_rule(RuleId::DAT_19)
            .with_help("write `not value` to negate it, or `~int(value)` for the int"),
        );
        Type::Error
    }

    /// Returns `value` as the `int` it is when it is a `bool`, as CPython does arithmetic on a
    /// `bool`, 0 or 1 (DAT-19, ADR-0098); any other value as it is.
    pub(super) fn int_of_bool(value: TypedExpr) -> TypedExpr {
        if value.ty != Type::Bool {
            return value;
        }
        let span = value.span;
        // A `True` or `False` written in the program is the constant 1 or 0, which an exponent
        // reads as a literal (EXP-04).
        if let TypedExprKind::Constant(TypedConstant::Bool(constant)) = value.kind {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::Int(u8::from(constant).to_string())),
                Type::Int,
                span,
            );
        }
        Self::typed(
            TypedExprKind::Builtin {
                builtin: crate::builtin::Builtin::Int,
                arguments: vec![value],
            },
            Type::Int,
            span,
        )
    }

    /// Returns the operands of `left op right` with each `bool` held as its `int`, as CPython
    /// adds, multiplies and divides them; `&`, `|` and `^` of two bools keep them, since they
    /// give a `bool` (DAT-03, DAT-19).
    pub(super) fn bool_operands(
        operator: BinaryOperator,
        left: TypedExpr,
        right: TypedExpr,
    ) -> (TypedExpr, TypedExpr) {
        let bitwise = matches!(
            operator,
            BinaryOperator::BitAnd | BinaryOperator::BitOr | BinaryOperator::BitXor
        );
        if bitwise && left.ty == Type::Bool && right.ty == Type::Bool {
            return (left, right);
        }
        // A `bool` is an `int` beside a number, and a repeat count beside a sequence; beside
        // any other value it stays a `bool`, which the error names.
        let takes_int = |other: &Type| {
            other.is_numeric_or_bool()
                || (operator == BinaryOperator::Mult && other.sequence_item().is_some())
                || (operator == BinaryOperator::Mult && matches!(other, Type::Str | Type::Bytes))
        };
        let left_takes = takes_int(&right.ty);
        let right_takes = takes_int(&left.ty);
        let left = if left_takes {
            Self::int_of_bool(left)
        } else {
            left
        };
        let right = if right_takes {
            Self::int_of_bool(right)
        } else {
            right
        };
        (left, right)
    }

    /// Returns the name of the class of an exception of `ty` that defines `__eq__` or `__ne__`,
    /// itself or through a base of the program, which `==` and `!=` call in CPython.
    fn exception_with_equality(&self, ty: &Type) -> Option<String> {
        let Type::Exception(exception) = ty else {
            return None;
        };
        let class = exception.class_id()?;
        ["__eq__", "__ne__"]
            .iter()
            .any(|name| self.program.method(class, name).is_some())
            .then(|| exception.name().to_owned())
    }

    pub(super) fn comparison_error(&mut self, left: &Type, right: &Type, span: Span) {
        let diagnostic = Diagnostic::new(
            DiagnosticCode::error(407),
            span,
            format!("{left} and {right} cannot be compared"),
        )
        .with_rule(RuleId::EXP_06);
        self.diagnostics
            .push(with_narrowing_help(diagnostic, left, right));
    }
}

/// Adds to an error about an optional operand how to use the value it holds (TYP-13), and
/// to one about a union how TyPy uses it member by member (TYP-14).
/// The help for an operation an `object` does not define (TYP-23).
pub(super) const OBJECT_HELP: &str =
    "an `object` is narrowed first, such as `if isinstance(value, int):`";

fn with_narrowing_help(diagnostic: Diagnostic, left: &Type, right: &Type) -> Diagnostic {
    if *left == Type::Object || *right == Type::Object {
        // An `object` does only what every value does; a test narrows it first (TYP-23).
        diagnostic.with_help(OBJECT_HELP)
    } else if matches!(left, Type::Optional(_)) || matches!(right, Type::Optional(_)) {
        diagnostic.with_help(
            "an optional value is used after a test shows it is not None, such as `if value is not None:`",
        )
    } else if matches!(left, Type::Union(_)) || matches!(right, Type::Union(_)) {
        diagnostic.with_help(
            "assign the union to a variable, which TyPy uses member by member, or narrow it \
             with `isinstance`",
        )
    } else {
        diagnostic
    }
}

fn binary_symbol(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "+",
        BinaryOperator::Sub => "-",
        BinaryOperator::Mult => "*",
        BinaryOperator::MatMult => "@",
        BinaryOperator::Div => "/",
        BinaryOperator::Mod => "%",
        BinaryOperator::Pow => "**",
        BinaryOperator::LShift => "<<",
        BinaryOperator::RShift => ">>",
        BinaryOperator::BitOr => "|",
        BinaryOperator::BitXor => "^",
        BinaryOperator::BitAnd => "&",
        BinaryOperator::FloorDiv => "//",
    }
}

impl BodyChecker<'_> {
    /// Checks `left is right`: two instances, or optional instances, are the same object or
    /// not. `True`, `False` and `None` are each the only object of its value, so `bool` and
    /// `bool | None` values compare by identity too (EXP-08), and two names of `typing` are
    /// each one object (TYP-18).
    fn identity_types(&mut self, (left, right): (&TypedExpr, &TypedExpr), span: Span) {
        let (l, r) = (&left.ty, &right.ty);
        if matches!(l, Type::Typing(_)) || matches!(r, Type::Typing(_)) {
            self.typing_identity(left, right, span);
            return;
        }
        let instance = |ty: &Type| match ty {
            Type::Optional(inner) => has_identity(inner),
            other => has_identity(other),
        };
        let singleton =
            |ty: &Type| *ty == Type::Bool || *ty == Type::Optional(Box::new(Type::Bool));
        let booleans = singleton(l) && singleton(r);
        // A method's result may be `NotImplemented`, which `is` finds (CLS-21).
        let giving_way = (*l == Type::NotImplemented && r.may_give_way())
            || (*r == Type::NotImplemented && l.may_give_way());
        if *l != Type::NoneType
            && *r != Type::NoneType
            && !booleans
            && !giving_way
            && !(instance(l) && instance(r))
        {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(301),
                    span,
                    format!("`is` compares identity, which is defined only against None, between bools or between objects here; these are {l} and {r}"),
                )
                .with_rule(RuleId::EXP_08)
                .with_help("use `==` to compare values"),
            );
        }
    }
}

/// Returns true when CPython defines the identity of every value of the type, as it does for
/// instances, exceptions, iterators, generators, modules, files, tracebacks and type objects,
/// and for a union of such types (EXP-08, TYP-14).
fn has_identity(ty: &Type) -> bool {
    match ty {
        Type::Union(members) => members.iter().all(has_identity),
        // An `object` is an object, and `is` compares two by their addresses (TYP-23).
        Type::Object => true,
        other => matches!(
            other,
            Type::Instance(..)
                | Type::Exception(_)
                | Type::Iterator(_)
                | Type::Generator { .. }
                | Type::Traceback
                | Type::Module(_)
                | Type::TypeAlias(_)
                | Type::VersionInfo
                | Type::NotImplemented
                | Type::AnyModule
                | Type::TextFile
                | Type::BinaryFile(_)
                | Type::DescriptorFile(_)
                | Type::CData(_)
                | Type::Callable { .. }
                | Type::TypeObject(_)
        ),
    }
}
