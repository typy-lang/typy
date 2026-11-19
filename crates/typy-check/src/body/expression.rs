//! Checks expressions and gives each its type.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};
use typy_source::syntax::{BoolOperator, Constant, Expr};

use super::BodyChecker;
use super::flow::test_facts;
use super::slice::Bounds;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks an expression; a `ctypes` class is refused unless the caller allowed it.
    pub(super) fn expression(&mut self, expression: &Expr) -> TypedExpr {
        let allowed = std::mem::take(&mut self.class_allowed);
        let typed = self.expression_or_class(expression);
        if allowed {
            return typed;
        }
        self.refuse_class(typed)
    }

    /// Checks an expression that may be a `ctypes` class, which a call, a `*`, `POINTER`
    /// and `sizeof` take (FFI-03).
    #[allow(clippy::too_many_lines, reason = "one arm per expression kind")]
    pub(super) fn expression_or_class(&mut self, expression: &Expr) -> TypedExpr {
        let span = expression.span();
        if let Some(each) = self.each_member(expression) {
            return each;
        }
        if let Some(folded) = self.version_info_use(expression) {
            return folded;
        }
        if let Some(part) = self.exception_part_use(expression) {
            return part;
        }
        match expression {
            Expr::Constant { value, .. } => {
                let constant = match value {
                    Constant::None => TypedConstant::None,
                    Constant::Bool(value) => TypedConstant::Bool(*value),
                    Constant::Int(digits) => TypedConstant::Int(digits.clone()),
                    Constant::Float(value) => TypedConstant::Float(*value),
                    Constant::Str(value) => TypedConstant::Str(value.clone()),
                    Constant::StrPoints(points) => TypedConstant::StrPoints(points.clone()),
                    Constant::Bytes(value) => TypedConstant::Bytes(value.clone()),
                    Constant::Complex { .. } | Constant::Ellipsis => {
                        return Self::typed(
                            TypedExprKind::Constant(TypedConstant::None),
                            Type::Error,
                            span,
                        );
                    }
                };
                let ty = constant.ty();
                self.program.makes_surrogates |= constant.holds_unescapable_surrogate();
                Self::typed(TypedExprKind::Constant(constant), ty, span)
            }
            Expr::Name { id, .. } => {
                let read = self.name(id, span);
                if matches!(read.ty, Type::ForeignLibrary(_)) {
                    return self.library_as_value(id, span);
                }
                read
            }
            Expr::BinOp {
                left,
                operator: typy_source::syntax::BinaryOperator::Mod,
                right,
                ..
            } if let Expr::Constant {
                value: typy_source::syntax::Constant::Str(format),
                span: format_span,
            } = &**left =>
            {
                self.percent_format(format, *format_span, right, span)
            }
            Expr::BinOp {
                left,
                operator,
                right,
                ..
            } => {
                let left = self.expression_or_class(left);
                let right = self.expression_or_class(right);
                // `T * length` and `length * T` make an array class (FFI-03).
                if *operator == typy_source::syntax::BinaryOperator::Mult {
                    if let Type::CDataClass(class) = left.ty.clone() {
                        return self.cdata_array_class(class, right, span);
                    }
                    if let Type::CDataClass(class) = right.ty.clone() {
                        return self.cdata_array_class(class, left, span);
                    }
                }
                let left = self.refuse_class(left);
                let right = self.refuse_class(right);
                let left = Self::version_tuple(left);
                let right = Self::version_tuple(right);
                // An instance's class may define the operator's method (CLS-07).
                let (left, right) = match self.operator_binary(*operator, left, right, span) {
                    super::operator_method::Comparison::Called(call) => return call,
                    super::operator_method::Comparison::Builtin(left, right) => (left, right),
                };
                let (left, right) = Self::tuple_operands(*operator, left, right);
                if let Some(formatted) = self.percent_dynamic(*operator, &left, &right, span) {
                    return formatted;
                }
                let ty = self.binary_type(*operator, &left, &right, span);
                Self::typed(
                    TypedExprKind::Binary {
                        left: Box::new(left),
                        operator: *operator,
                        right: Box::new(right),
                    },
                    ty,
                    span,
                )
            }
            Expr::UnaryOp {
                operator, operand, ..
            } => {
                let operand = self.expression(operand);
                if let Some(call) = super::operator_method::unary_name(*operator)
                    .and_then(|name| self.operator_unary(name, &operand, span))
                {
                    return call;
                }
                let ty = self.unary_type(*operator, &operand, span);
                Self::typed(
                    TypedExprKind::Unary {
                        operator: *operator,
                        operand: Box::new(operand),
                    },
                    ty,
                    span,
                )
            }
            Expr::BoolOp {
                operator, values, ..
            } => {
                // Each operand is read only when the ones before it held for `and`, or failed
                // for `or`, so it is checked with what that shows (TYP-13).
                let outer = self.assigned.clone();
                let values: Vec<TypedExpr> = values
                    .iter()
                    .map(|value| {
                        let value = self.truth_test(value);
                        let (holds, fails) = test_facts(&value);
                        self.add_facts(if *operator == BoolOperator::And {
                            holds
                        } else {
                            fails
                        });
                        value
                    })
                    .collect();
                self.assigned = outer;
                let ty = values[0].ty.clone();
                if let Some(other) = values.iter().find(|value| !value.ty.is_assignable_to(&ty)) {
                    let word = if *operator == BoolOperator::And {
                        "and"
                    } else {
                        "or"
                    };
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(303),
                            other.span,
                            format!(
                                "the operands of `{word}` must have one type; they are {ty} and {}",
                                other.ty
                            ),
                        )
                        .with_rule(RuleId::EXP_09),
                        span,
                    );
                }
                Self::typed(
                    TypedExprKind::BoolOp {
                        operator: *operator,
                        values,
                    },
                    ty,
                    span,
                )
            }
            Expr::Compare {
                left,
                operators,
                comparators,
                ..
            } => {
                let mut operands = self.comparison_operands(left, comparators, operators);
                let mut comparators = operands.split_off(1);
                let mut left = operands.remove(0);
                // `sys.version_info` held as a value compares as CPython compares it (LIB-16).
                if let ([operator], [right]) = (operators.as_slice(), comparators.as_slice())
                    && let Some(folded) = Self::version_comparison(*operator, &left, right, span)
                {
                    return folded;
                }
                // A comparison of instances may call a comparison method (CLS-07).
                if let ([operator], [right]) = (operators.as_slice(), comparators.as_slice()) {
                    let right = right.clone();
                    match self.operator_comparison(*operator, left, right, span) {
                        super::operator_method::Comparison::Called(call) => return call,
                        super::operator_method::Comparison::Builtin(back, other) => {
                            left = back;
                            comparators = vec![other];
                        }
                    }
                } else if std::iter::once(&left).chain(&comparators).any(|operand| {
                    matches!(&operand.ty, Type::Instance(class)
                        if class.operators.compares() || class.operators.orders())
                }) {
                    // A chain of such comparisons joins its comparisons with `and` (CLS-22).
                    return self.instance_chain(left, operators, comparators, span);
                }
                let mut previous = &left;
                for (operator, right) in operators.iter().zip(&comparators) {
                    self.compare_types(*operator, previous, right);
                    previous = right;
                }
                Self::typed(
                    TypedExprKind::Compare {
                        left: Box::new(left),
                        operators: operators.clone(),
                        comparators,
                    },
                    Type::Bool,
                    span,
                )
            }
            Expr::Named { target, value, .. } => self.named(target, value, span),
            Expr::Lambda { .. } => self.lambda(expression, super::lambda::Expected::default()),
            Expr::IfExp {
                test, body, orelse, ..
            } => self.conditional((test, body, orelse), None, span),
            Expr::Set { elements, .. } => self.set_display(elements, None, span),
            Expr::Dict { keys, values, .. } => self.dict_display(keys, values, None, span),
            Expr::ListComp { .. } | Expr::SetComp { .. } | Expr::DictComp { .. } => {
                self.comprehension(expression, None)
            }
            Expr::Tuple { elements, .. } => {
                let elements: Vec<TypedExpr> = elements
                    .iter()
                    .map(|element| self.expression(element))
                    .collect();
                let ty = Type::Tuple(elements.iter().map(|element| element.ty.clone()).collect());
                Self::typed(TypedExprKind::Tuple(elements), ty, span)
            }
            Expr::Subscript { value, index, .. } => {
                let value = Self::version_tuple(self.expression(value));
                if let Type::CData(class) = value.ty.clone() {
                    return self.cdata_item((value, &class), index, span);
                }
                // An instance's class may define `__getitem__` (CLS-07).
                let value = match self.item_call(value, index, span) {
                    Ok(call) => return call,
                    Err(value) => value,
                };
                if let Expr::Slice {
                    lower, upper, step, ..
                } = index.as_ref()
                {
                    let bounds = Bounds {
                        lower: lower.as_deref(),
                        upper: upper.as_deref(),
                        step: step.as_deref(),
                    };
                    return self.slice(value, &bounds, span);
                }
                if let Type::Tuple(items) = &value.ty {
                    let items = items.clone();
                    return self.tuple_item(value, &items, index, span);
                }
                // A slice object slices a built-in sequence as its bounds do (CLS-23).
                let typed_index = if matches!(
                    value.ty,
                    Type::Str | Type::Bytes | Type::List(_) | Type::TupleOf(_) | Type::Range
                ) {
                    let index = self.expression(index);
                    if index.ty == Type::Slice {
                        return self.sliced_by_object(value, index, span);
                    }
                    Some(index)
                } else {
                    None
                };
                if let Some(item) = value.ty.sequence_item() {
                    let item = item.clone();
                    let index = typed_index.unwrap_or_else(|| self.expression(index));
                    return self.list_item_of(value, item, index, span);
                }
                if let Type::Dict(key, entry) = &value.ty {
                    let (key, entry) = (key.as_ref().clone(), entry.as_ref().clone());
                    return self.dict_item(value, &key, entry, index, span);
                }
                let index = typed_index.unwrap_or_else(|| self.expression(index));
                if value.ty == Type::Error || index.ty == Type::Error {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                if matches!(value.ty, Type::Bytes | Type::Range) {
                    self.expect(&index, Type::Int, "an index");
                    return Self::typed(
                        TypedExprKind::Index {
                            value: Box::new(value),
                            index: Box::new(index),
                        },
                        Type::Int,
                        span,
                    );
                }
                // CPython gives a generic alias, a value of `typing` (TYP-18).
                if value.ty == Type::TypeAlias {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(201),
                            span,
                            "a type alias given its type arguments as a value is planned for \
                             TyPy 0.9 (TYP-18)",
                        )
                        .with_rule(RuleId::TYP_18),
                        span,
                    );
                }
                if value.ty != Type::Str {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(401),
                            value.span,
                            format!("{} cannot be indexed", value.ty.with_article()),
                        )
                        .with_rule(RuleId::EXP_02),
                        span,
                    );
                }
                self.expect(&index, Type::Int, "an index");
                Self::typed(
                    TypedExprKind::Index {
                        value: Box::new(value),
                        index: Box::new(index),
                    },
                    Type::Str,
                    span,
                )
            }
            Expr::Call {
                function,
                arguments,
                keywords,
                ..
            } => match function.as_ref() {
                // A module's declaration of a library holds nothing to evaluate (FFI-01).
                _ if let Some(library) = self.foreign_library(span) => library,
                Expr::Name {
                    id,
                    span: name_span,
                } => self.call(id, *name_span, arguments, keywords, span),
                Expr::Attribute {
                    value, attribute, ..
                } => self.method_call(value, attribute, arguments, keywords, span),
                // A generic class given its type arguments, such as `Box[int](1)`.
                Expr::Subscript { value, .. }
                    if matches!(value.as_ref(), Expr::Name { id, .. }
                        if self.program.class_ids.contains_key(id)) =>
                {
                    self.applied_class_call(function, arguments, keywords, span)
                }
                // A class `POINTER(T)` or `T * length` called where it is made (FFI-03), or a
                // function value (FUN-05).
                _ => {
                    let class = self.expression_or_class(function);
                    match class.ty {
                        Type::CDataClass(class) => {
                            self.cdata_new(&class, arguments, keywords, span)
                        }
                        Type::Error => {
                            for argument in arguments {
                                self.expression(argument);
                            }
                            TypedExpr { span, ..class }
                        }
                        Type::Callable { .. } => self.call_value(class, arguments, keywords, span),
                        _ => {
                            for argument in arguments {
                                self.expression(argument);
                            }
                            self.error(
                                Diagnostic::new(
                                    DiagnosticCode::error(401),
                                    function.span(),
                                    format!("{} cannot be called", class.ty.with_article()),
                                )
                                .with_rule(RuleId::EXP_02),
                                span,
                            )
                        }
                    }
                }
            },
            Expr::List { elements, .. } => self.list_display(elements, None, span),
            Expr::Slice { .. } => self.misplaced_slice(span),
            Expr::Attribute {
                value, attribute, ..
            } => self.attribute(value, attribute, span),
            Expr::FString { parts, .. } => self.fstring(parts, span),
            // The subset check refuses every other expression, and a quoted expression is
            // only read as an annotation.
            Expr::Quoted { .. } | Expr::Other { .. } => Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            ),
        }
    }

    pub(super) fn truth_test(&mut self, test: &Expr) -> TypedExpr {
        if let Some(truth) = self.tested_part(test) {
            return truth;
        }
        let test = self.expression(test);
        self.check_truth(&test);
        test
    }

    /// Reports a value whose truth cannot be tested: a condition, `not`, `and`, `or` and
    /// `bool()` all test it (EXP-10).
    pub(super) fn check_truth(&mut self, value: &TypedExpr) {
        // An instance's truth comes from the `__bool__` or `__len__` of the object's own class,
        // which the lowering asks through its type object when a subclass may decide (CLS-24).
        if !value.ty.is_truth_testable() {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(304),
                    value.span,
                    format!("{} cannot be used as a condition", value.ty.with_article()),
                )
                .with_rule(RuleId::EXP_10),
            );
        }
    }

    /// Checks `tuple[index]`: the position is an integer literal, counted from the end when
    /// negative, inside the tuple, so the item and its type are known when compiling (DAT-08).
    fn tuple_item(
        &mut self,
        tuple: TypedExpr,
        items: &[Type],
        index: &Expr,
        span: typy_diagnostics::Span,
    ) -> TypedExpr {
        let Some(position) = literal_position(index) else {
            let index = self.expression(index);
            if index.ty.has_error() || tuple.ty.has_error() {
                return Self::typed(
                    TypedExprKind::Constant(TypedConstant::None),
                    Type::Error,
                    span,
                );
            }
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(307),
                    index.span,
                    "a tuple is indexed with an integer literal, because each item can have its own type",
                )
                .with_rule(RuleId::DAT_08)
                .with_help("write the position as a number, such as `pair[0]`"),
                span,
            );
        };
        let length = i128::try_from(items.len()).unwrap_or(i128::MAX);
        let from_start = if position < 0 {
            position + length
        } else {
            position
        };
        let Some(index_from_start) = usize::try_from(from_start)
            .ok()
            .filter(|index| *index < items.len())
        else {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(307),
                    index.span(),
                    format!(
                        "position {position} is outside {}, which has {} items",
                        tuple.ty,
                        items.len()
                    ),
                )
                .with_rule(RuleId::DAT_08),
                span,
            );
        };
        let ty = items[index_from_start].clone();
        Self::typed(
            TypedExprKind::TupleItem {
                tuple: Box::new(tuple),
                index: index_from_start,
            },
            ty,
            span,
        )
    }

    /// Reports a value whose type is not `expected`.
    pub(super) fn expect(&mut self, value: &TypedExpr, expected: Type, what: &str) {
        // A type that holds an error was already reported where the error arose.
        if value.ty.has_error() || value.ty.is_assignable_to(&expected) {
            return;
        }
        let rule = if expected == Type::Float && value.ty == Type::Int {
            RuleId::TYP_09
        } else {
            RuleId::TYP_01
        };
        let mut diagnostic = Diagnostic::new(
            DiagnosticCode::error(404),
            value.span,
            format!(
                "{what} must be {}, and this is {}",
                expected.with_article(),
                value.ty.with_article()
            ),
        )
        .with_rule(rule);
        if let Some(help) = self.protocol_help(&value.ty, &expected) {
            diagnostic = diagnostic.with_help(help);
        }
        if let (TypedExprKind::Constant(TypedConstant::Int(digits)), Type::Float) =
            (&value.kind, expected)
        {
            diagnostic = diagnostic.with_fix(value.span, format!("{digits}.0"));
        }
        self.diagnostics.push(diagnostic);
    }
}

/// Returns the value of an integer literal, or of a negated one, such as `2` or `-1`.
pub(super) fn literal_position(index: &Expr) -> Option<i128> {
    use typy_source::syntax::UnaryOperator;
    match index {
        Expr::Constant {
            value: Constant::Int(digits),
            ..
        } => digits.parse().ok().or(Some(i128::MAX)),
        Expr::UnaryOp {
            operator: UnaryOperator::USub,
            operand,
            ..
        } => literal_position(operand).map(|value| -value),
        _ => None,
    }
}
