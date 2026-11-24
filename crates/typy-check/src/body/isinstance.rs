//! Checks `isinstance(value, classes)` and the type it narrows the value to (CLS-09, TYP-07,
//! ADR-0021).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{CompareOperator, Expr, Keyword};

use super::BodyChecker;
use crate::class_type::InstanceClass;
use crate::typed::{IsInstanceTest, MemberTest, RuntimeKind, TypedExpr, TypedExprKind};
use crate::types::{AbstractClass, Type, TypeObjectOf};

/// What a test of a value against one class gives, as far as the checker can tell.
enum Outcome {
    /// Every value of the static type is an instance of the class.
    Always,
    /// No value of the static type is.
    Never,
    /// The value's class decides, when the program runs.
    Checked,
}

impl BodyChecker<'_> {
    /// Checks `isinstance(value, class)` with a class, or a tuple of classes, named as values:
    /// a class of the program or a built-in type or exception class.
    /// Checks `callable(x)` (LIB-23): true for a function value and a class, false for any
    /// other value, as TyPy has no `__call__`; tested when the program runs only for an
    /// optional or a union, as `isinstance` tests one, and `x` is evaluated either way.
    pub(super) fn call_callable(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let ([value], []) = (arguments, keywords) else {
            for argument in arguments {
                self.expression(argument);
            }
            let message = if keywords.is_empty() {
                format!(
                    "callable() takes exactly one argument ({} given)",
                    arguments.len()
                )
            } else {
                "callable() takes no keyword arguments".to_owned()
            };
            return self.error(
                Diagnostic::new(DiagnosticCode::error(404), span, message)
                    .with_rule(RuleId::LIB_23),
                span,
            );
        };
        // A built-in function, method or type is callable, and reading it has no effect.
        if self.names_builtin(value) {
            return Self::typed(
                TypedExprKind::Constant(crate::typed::TypedConstant::Bool(true)),
                Type::Bool,
                span,
            );
        }
        let value = self.expression(value);
        let calls = |ty: &Type| matches!(ty, Type::Callable { .. } | Type::TypeObject(_));
        let (always, members) = match &value.ty {
            Type::Optional(inner) => (calls(inner), Vec::new()),
            Type::Union(members) => (
                false,
                members
                    .iter()
                    .map(|member| (calls(member), Vec::new()))
                    .collect(),
            ),
            other => (calls(other), Vec::new()),
        };
        let test = crate::typed::IsInstanceTest {
            value,
            none_matches: false,
            always,
            checks: Vec::new(),
            narrowed: None,
            otherwise: None,
            members,
            held: Vec::new(),
            guard: None,
        };
        Self::typed(TypedExprKind::IsInstance(Box::new(test)), Type::Bool, span)
    }

    /// Checks `issubclass(class, classes)` (CLS-19, ADR-0084): the class, and each class of
    /// the second argument, a class or a tuple of them, named or held in a value, are type
    /// objects the runtime tests in order. A value that is no class is E0401, as CPython
    /// raises `TypeError`.
    pub(super) fn call_issubclass(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let ([class, classes], []) = (arguments, keywords) else {
            for argument in arguments {
                self.expression(argument);
            }
            let message = if keywords.is_empty() {
                format!("issubclass expected 2 arguments, got {}", arguments.len())
            } else {
                "issubclass() takes no keyword arguments".to_owned()
            };
            return self.error(
                Diagnostic::new(DiagnosticCode::error(404), span, message)
                    .with_rule(RuleId::CLS_19),
                span,
            );
        };
        let class = self.expression(class);
        let named = match classes {
            Expr::Tuple { elements, .. } => elements.iter().collect(),
            other => vec![other],
        };
        let bases: Vec<TypedExpr> = named.iter().map(|each| self.expression(each)).collect();
        let not_class = |value: &TypedExpr| {
            !matches!(value.ty, Type::TypeObject(_) | Type::Error) && !holds_classes(&value.ty)
        };
        if not_class(&class) {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    class.span,
                    "issubclass() arg 1 must be a class: CPython raises `TypeError`",
                )
                .with_rule(RuleId::CLS_19),
                span,
            );
        }
        if let Some(base) = bases.iter().find(|base| not_class(base)) {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    base.span,
                    "issubclass() arg 2 must be a class, a tuple of classes, or a union: CPython \
                     raises `TypeError`",
                )
                .with_rule(RuleId::CLS_19),
                span,
            );
        }
        Self::typed(
            TypedExprKind::IsSubclass {
                class: Box::new(class),
                bases,
            },
            Type::Bool,
            span,
        )
    }

    pub(super) fn call_isinstance(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let ([value, classes], []) = (arguments, keywords) else {
            for argument in arguments {
                self.expression(argument);
            }
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    "`isinstance` takes a value and a class or a tuple of classes",
                )
                .with_rule(RuleId::CLS_09),
                span,
            );
        };
        let value = self.expression(value);
        let mut held = Vec::new();
        let classes = match self.isinstance_classes_holding(classes, &mut held) {
            Ok(classes) => classes,
            Err(Some(diagnostic)) => return self.error(*diagnostic, span),
            // The class is reported where it is named.
            Err(None) => {
                return Self::typed(
                    TypedExprKind::Constant(crate::typed::TypedConstant::None),
                    Type::Error,
                    span,
                );
            }
        };
        let test = self.isinstance_test(value, &classes, span);
        self.with_held_classes(test, held)
    }

    /// Returns the test `test` that also asks about `held`, classes held in values, whose
    /// bound, the class their type names, narrows the value as a named class does (CLS-18).
    pub(super) fn with_held_classes(&mut self, test: TypedExpr, held: Vec<TypedExpr>) -> TypedExpr {
        if held.is_empty() {
            return test;
        }
        let TypedExprKind::IsInstance(mut test_parts) = test.kind else {
            return test;
        };
        // A class read when the program runs narrows only to the class its type bounds it by,
        // and a tuple of them to the classes its items' types bound them by.
        let bounds: Option<Vec<TypeObjectOf>> = held
            .iter()
            .flat_map(|class| match &class.ty {
                Type::Tuple(items) => items.iter().collect(),
                Type::TupleOf(item) => vec![item.as_ref()],
                other => vec![other],
            })
            .map(|ty| match ty {
                Type::TypeObject(Some(of)) => Some(of.as_ref().clone()),
                _ => None,
            })
            .collect();
        if test_parts.checks.is_empty()
            && !test_parts.always
            && let Some(bounds) = bounds
        {
            let inner = match &test_parts.value.ty {
                Type::Optional(inner) => inner.as_ref().clone(),
                other => other.clone(),
            };
            let value = test_parts.value.clone();
            test_parts.narrowed = self.narrowing(&value, &inner, &bounds, false, false);
        } else {
            test_parts.narrowed = None;
            test_parts.otherwise = None;
        }
        test_parts.held = held;
        Self::typed(TypedExprKind::IsInstance(test_parts), Type::Bool, test.span)
    }

    /// Returns the test `isinstance(value, classes)` with the classes known, and what it
    /// narrows the value to; a `match` pattern tests its subject this way too (CST-06).
    pub(super) fn isinstance_test(
        &mut self,
        value: TypedExpr,
        classes: &[TypeObjectOf],
        span: Span,
    ) -> TypedExpr {
        let inner = match &value.ty {
            Type::Optional(inner) => inner.as_ref().clone(),
            other => other.clone(),
        };
        let none_matches = matches!(value.ty, Type::Optional(_))
            && classes
                .iter()
                .any(|class| matches!(class, TypeObjectOf::NoneType | TypeObjectOf::Object));
        let outcomes: Vec<Outcome> = classes.iter().map(|class| outcome(&inner, class)).collect();
        let always = outcomes.iter().any(|each| matches!(each, Outcome::Always));
        let checks: Vec<MemberTest> = classes
            .iter()
            .zip(&outcomes)
            .filter(|(_, outcome)| matches!(outcome, Outcome::Checked))
            .flat_map(|(class, _)| self.concrete_classes(class))
            .collect();
        let narrowed = self.narrowing(&value, &inner, classes, always, none_matches);
        let otherwise = self.union_otherwise(&value, &inner, classes);
        let members = match &inner {
            Type::Union(members) => members
                .iter()
                .map(|member| {
                    let (always, checks) = member_outcome(member, classes);
                    let checks = checks
                        .iter()
                        .flat_map(|class| self.concrete_classes(class))
                        .collect();
                    (always, checks)
                })
                .collect(),
            _ => Vec::new(),
        };
        Self::typed(
            TypedExprKind::IsInstance(Box::new(IsInstanceTest {
                value,
                none_matches,
                always,
                checks,
                narrowed,
                otherwise,
                members,
                held: Vec::new(),
                guard: None,
            })),
            Type::Bool,
            span,
        )
    }

    /// Returns the classes a test against `class` asks about when the program runs: the class
    /// itself, or for `Iterable` and `Iterator` the classes that define their methods (TYP-17).
    fn concrete_classes(&self, class: &TypeObjectOf) -> Vec<MemberTest> {
        let TypeObjectOf::Abstract { kind, implementers } = class else {
            return vec![MemberTest::Class(class.clone())];
        };
        let mut tests: Vec<MemberTest> = implementers
            .iter()
            .map(|(id, _)| {
                let name = self.program.classes[id.0 as usize].name.clone();
                MemberTest::Class(TypeObjectOf::Instance(*id, name))
            })
            .collect();
        // The runtime's own iterators, its generators and files are iterators, and its
        // generators are generators, as their objects' headers say (TYP-17, TYP-21).
        match kind {
            AbstractClass::Iterator => tests.extend([
                MemberTest::Kind(RuntimeKind::Iterator),
                MemberTest::Kind(RuntimeKind::Generator),
                MemberTest::Kind(RuntimeKind::File),
            ]),
            AbstractClass::Generator => tests.push(MemberTest::Kind(RuntimeKind::Generator)),
            AbstractClass::Iterable => {}
            // The built-in collections are told apart by their classes (LIB-20).
            AbstractClass::Container | AbstractClass::Collection | AbstractClass::Reversible => {
                use crate::types::View;
                let mut builtin = vec![
                    TypeObjectOf::List,
                    TypeObjectOf::Tuple,
                    TypeObjectOf::Str,
                    TypeObjectOf::Bytes,
                    TypeObjectOf::Bytearray,
                    TypeObjectOf::Range,
                    TypeObjectOf::Dict,
                    TypeObjectOf::View(View::Keys),
                    TypeObjectOf::View(View::Values),
                    TypeObjectOf::View(View::Items),
                ];
                if *kind != AbstractClass::Reversible {
                    builtin.push(TypeObjectOf::Set);
                }
                tests.extend(builtin.into_iter().map(MemberTest::Class));
            }
        }
        tests
    }

    /// Returns the classes the second argument names, a class or a tuple of them, and puts
    /// each class held in a value into `held`, which is tested when the program runs (CLS-18).
    pub(super) fn isinstance_classes_holding(
        &mut self,
        classes: &Expr,
        held: &mut Vec<TypedExpr>,
    ) -> Result<Vec<TypeObjectOf>, Option<Box<Diagnostic>>> {
        let named = match classes {
            Expr::Tuple { elements, .. } => elements.iter().collect(),
            other => vec![other],
        };
        let mut found = Vec::new();
        for each in named {
            if let Some(classes) = self.typing_classes(each, held)? {
                found.extend(classes);
                continue;
            }
            match self.named_class(each)? {
                Named::Class(class) => found.push(class),
                Named::Held(value) => held.push(value),
            }
        }
        Ok(found)
    }

    /// Returns the class an expression names as a value: a class of the program, or a
    /// built-in type or exception class.
    fn named_class(&mut self, each: &Expr) -> Result<Named, Option<Box<Diagnostic>>> {
        let typed = self.expression(each);
        if matches!(typed.ty, Type::TypeAlias(_)) {
            return Err(Some(Box::new(
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    each.span(),
                    "isinstance() arg 2 must be a type, a tuple of types, or a union: CPython \
                     raises `TypeError`",
                )
                .with_rule(RuleId::SMT_12),
            )));
        }
        // CPython raises `TypeError` for a protocol not marked `@runtime_checkable`,
        // and TyPy keeps no members to test at run time.
        if let TypedExprKind::TypeObject(of) = &typed.kind
            && let TypeObjectOf::Instance(class, name) = of.as_ref()
            && self.program.is_protocol(*class)
        {
            return Err(Some(Box::new(
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    each.span(),
                    format!(
                        "`isinstance` with the protocol `{name}` raises `TypeError` in \
                         CPython"
                    ),
                )
                .with_rule(RuleId::GEN_05)
                .with_help("test for a class that satisfies the protocol"),
            )));
        }
        match typed.kind {
            TypedExprKind::TypeObject(of) => Ok(Named::Class(*of)),
            _ if typed.ty == Type::Error => Err(None),
            // A class held in a value is tested when the program runs (CLS-18).
            _ if matches!(typed.ty, Type::TypeObject(_)) || holds_classes(&typed.ty) => {
                Ok(Named::Held(typed))
            }
            // Any other value is no class: CPython raises `TypeError` for it.
            _ => Err(Some(Box::new(
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    each.span(),
                    "isinstance() arg 2 must be a type, a tuple of types, or a union: CPython \
                     raises `TypeError`",
                )
                .with_rule(RuleId::CLS_09),
            ))),
        }
    }

    /// Returns the position, in the function's table of narrowed types, of the type a true
    /// test gives a variable: the nearest class every tested class of the program derives
    /// from, when each derives from the variable's static class.
    fn narrowing(
        &mut self,
        value: &TypedExpr,
        inner: &Type,
        classes: &[TypeObjectOf],
        always: bool,
        none_matches: bool,
    ) -> Option<u32> {
        if none_matches || super::flow::narrowable_variable(value).is_none() {
            return None;
        }
        let narrowed = if always {
            inner.clone()
        } else if let Type::Union(members) = inner {
            let kept: Vec<Type> = members
                .iter()
                .flat_map(|member| self.member_when_true(member, classes))
                .collect();
            if kept.is_empty() {
                return None;
            }
            Type::union(kept)
        } else if *inner == Type::Object {
            self.object_narrowed(classes, value.span)?
        } else if let Type::Exception(own) = inner {
            let mut tested = Vec::new();
            for class in classes {
                match class {
                    TypeObjectOf::Exception(each) if each.derives_from(own) => {
                        tested.push(each.clone());
                    }
                    _ => return None,
                }
            }
            if tested.is_empty() {
                return None;
            }
            Type::Exception(crate::exception::common_base(&tested))
        } else if let Type::Iterable(item) = inner
            && classes.iter().all(|class| {
                matches!(
                    class,
                    TypeObjectOf::Abstract {
                        kind: AbstractClass::Iterator | AbstractClass::Generator,
                        ..
                    }
                )
            })
        {
            // An iterable found an iterator, or a generator, is read as an iterator (TYP-21).
            Type::Iterator(item.clone())
        } else {
            let Type::Instance(own) = inner else {
                return None;
            };
            let mut common: Option<InstanceClass> = None;
            for class in classes {
                let TypeObjectOf::Instance(id, _) = class else {
                    return None;
                };
                // A generic class has no instances of its own: the variable keeps its type, as
                // the copy the object belongs to is known only when the program runs (GEN-14).
                if self.program.is_opaque_class(*id) {
                    return None;
                }
                let Type::Instance(tested) = self.program.instance_type(*id) else {
                    return None;
                };
                if !tested.derives_from(own) {
                    return None;
                }
                common = Some(match common {
                    None => tested.as_ref().clone(),
                    Some(common) => common.common_base(&tested)?,
                });
            }
            Type::instance(common?)
        };
        self.narrowing_position(value, narrowed)
    }

    /// Returns `type(x) is C`, or `is not`, for a variable `x` of type `object` and a class
    /// `C` the program names, as a test that narrows `x` to `C` when it holds (TYP-23); `None`
    /// for any other comparison.
    pub(super) fn type_is(
        &mut self,
        operator: CompareOperator,
        left: &TypedExpr,
        right: &TypedExpr,
        span: Span,
    ) -> Option<TypedExpr> {
        let negated = match operator {
            CompareOperator::Is => false,
            CompareOperator::IsNot => true,
            _ => return None,
        };
        let ((TypedExprKind::TypeOf(value), TypedExprKind::TypeObject(class))
        | (TypedExprKind::TypeObject(class), TypedExprKind::TypeOf(value))) =
            (&left.kind, &right.kind)
        else {
            return None;
        };
        if value.ty != Type::Object || super::flow::narrowable_variable(value).is_none() {
            return None;
        }
        let class = class.as_ref().clone();
        let exact = match &class {
            TypeObjectOf::Int => Some(Type::Int),
            _ => self.object_narrowed(std::slice::from_ref(&class), value.span),
        };
        let narrowed = exact.map(|narrowed| self.narrowing_index(narrowed));
        let test = Self::typed(
            TypedExprKind::IsInstance(Box::new(IsInstanceTest {
                value: value.as_ref().clone(),
                none_matches: false,
                always: false,
                checks: vec![MemberTest::Exact(class)],
                narrowed,
                otherwise: None,
                members: Vec::new(),
                held: Vec::new(),
                guard: None,
            })),
            Type::Bool,
            span,
        );
        Some(if negated {
            Self::typed(
                TypedExprKind::Unary {
                    operator: typy_source::syntax::UnaryOperator::Not,
                    operand: Box::new(test),
                },
                Type::Bool,
                span,
            )
        } else {
            test
        })
    }

    /// Returns the type a true test gives an `object`: the union of the tested classes'
    /// types, `int` taking `bool` too, as a `bool` is an `int` in CPython and stays a `bool`
    /// (TYP-23, ADR-0090). A built-in collection or an abstract one is reported: its items'
    /// type would be `Any`, which is planned (TYP-25), and TyPy holds the items of a collection
    /// by their own type, so no other item type reads them safely.
    fn object_narrowed(&mut self, classes: &[TypeObjectOf], span: Span) -> Option<Type> {
        let mut found = Vec::new();
        for class in classes {
            found.extend(match class {
                TypeObjectOf::Object => return None,
                TypeObjectOf::Int => vec![Type::Int, Type::Bool],
                TypeObjectOf::Float => vec![Type::Float],
                TypeObjectOf::Bool => vec![Type::Bool],
                TypeObjectOf::Str => vec![Type::Str],
                TypeObjectOf::Bytes => vec![Type::Bytes],
                TypeObjectOf::Bytearray => vec![Type::Bytearray],
                TypeObjectOf::NoneType => vec![Type::NoneType],
                TypeObjectOf::Type => vec![Type::TypeObject(None)],
                TypeObjectOf::Range => vec![Type::Range],
                TypeObjectOf::Slice => vec![Type::Slice],
                TypeObjectOf::Exception(exception) => vec![Type::Exception(exception.clone())],
                // A generic class has no instances of its own (GEN-14).
                TypeObjectOf::Instance(id, _) if self.program.is_opaque_class(*id) => {
                    return None;
                }
                TypeObjectOf::Instance(id, _) => vec![self.program.instance_type(*id)],
                collection => {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(201),
                            span,
                            format!(
                                "narrowing an `object` to `{}` needs the type of its items, \
                                 `Any`, which is planned for TyPy 0.13 (TYP-25)",
                                collection.name()
                            ),
                        )
                        .with_rule(RuleId::TYP_25)
                        .with_help("hold the collection in a variable of its own type instead"),
                    );
                    return None;
                }
            });
        }
        Some(Type::union(found))
    }

    /// Returns the position of `narrowed` in the function's table of narrowed types, unless
    /// it is the value's own type.
    fn narrowing_position(&mut self, value: &TypedExpr, narrowed: Type) -> Option<u32> {
        if narrowed == value.ty {
            return None;
        }
        Some(self.narrowing_index(narrowed))
    }

    /// Returns the position of `narrowed` in the function's table of narrowed types, adding
    /// it when it is not there yet.
    pub(super) fn narrowing_index(&mut self, narrowed: Type) -> u32 {
        let position = self
            .narrowings
            .iter()
            .position(|each| *each == narrowed)
            .unwrap_or_else(|| {
                self.narrowings.push(narrowed);
                self.narrowings.len() - 1
            });
        // A function has far fewer narrowed types than `u32::MAX`.
        u32::try_from(position).unwrap_or(u32::MAX)
    }

    /// Returns what a member of a union may be when the test holds: the member, when a tested
    /// class takes all of it or may take part, or the tested classes derived from it, when
    /// only they can.
    fn member_when_true(&self, member: &Type, classes: &[TypeObjectOf]) -> Vec<Type> {
        let mut found = Vec::new();
        for class in classes {
            match outcome(member, class) {
                Outcome::Always => return vec![member.clone()],
                Outcome::Never => {}
                Outcome::Checked => match (member, class) {
                    (Type::Instance(_), TypeObjectOf::Instance(id, _))
                        if self.program.is_opaque_class(*id) =>
                    {
                        return vec![member.clone()];
                    }
                    (Type::Instance(own), TypeObjectOf::Instance(id, _))
                        if matches!(self.program.instance_type(*id),
                            Type::Instance(tested) if tested.derives_from(own)) =>
                    {
                        found.push(self.program.instance_type(*id));
                    }
                    // With single inheritance, an unrelated class takes no instance.
                    (Type::Instance(_), TypeObjectOf::Instance(..)) => {}
                    _ => return vec![member.clone()],
                },
            }
        }
        found
    }

    /// Returns the position of the type a false test gives a variable that holds a union: the
    /// members no tested class takes in every case, or `None` when that is every member or no
    /// member.
    fn union_otherwise(
        &mut self,
        value: &TypedExpr,
        inner: &Type,
        classes: &[TypeObjectOf],
    ) -> Option<u32> {
        let Type::Union(members) = inner else {
            return None;
        };
        super::flow::narrowable_variable(value)?;
        let left: Vec<Type> = members
            .iter()
            .filter(|member| {
                !classes
                    .iter()
                    .any(|class| matches!(outcome(member, class), Outcome::Always))
            })
            .cloned()
            .collect();
        if left.is_empty() {
            return None;
        }
        // `None` is among the classes only with `object`, which takes every member, so a
        // value that may be `None` still may here.
        let rest = Type::union(left);
        let rest = if matches!(value.ty, Type::Optional(_)) {
            Type::optional(rest)
        } else {
            rest
        };
        self.narrowing_position(value, rest)
    }
}

/// Returns, for a member of a union, whether every value of it is an instance of one of the
/// classes, and the classes its value's class decides.
fn member_outcome(member: &Type, classes: &[TypeObjectOf]) -> (bool, Vec<TypeObjectOf>) {
    let outcomes: Vec<Outcome> = classes.iter().map(|class| outcome(member, class)).collect();
    let always = outcomes.iter().any(|each| matches!(each, Outcome::Always));
    let checks = classes
        .iter()
        .zip(&outcomes)
        .filter(|(_, each)| matches!(each, Outcome::Checked))
        .map(|(class, _)| class.clone())
        .collect();
    (always, checks)
}

/// Returns whether every value of type `ty`, which is not optional, is an instance of
/// `class`.
pub(super) fn always_instance(ty: &Type, class: &TypeObjectOf) -> bool {
    matches!(outcome(ty, class), Outcome::Always)
}

/// Returns what testing a value of type `ty`, which is not optional and not a union, against
/// `Iterable` or `Iterator` gives, as CPython's `__subclasshook__` answers (TYP-17): the
/// built-in collections are iterable, their iterators and files are iterators too, and a class
/// of the program is one when it or a base defines the methods, or may be one when only a
/// subclass does.
fn abstract_outcome(
    ty: &Type,
    kind: crate::types::AbstractClass,
    implementers: &[(crate::typed::ClassId, Vec<crate::typed::ClassId>)],
) -> Outcome {
    use crate::types::AbstractClass;
    let always = |yes: bool| if yes { Outcome::Always } else { Outcome::Never };
    // `Container`, `Collection` and `Reversible` take the built-in collections CPython
    // registers with them, and no iterator or file (LIB-20).
    let collection = matches!(
        kind,
        AbstractClass::Container | AbstractClass::Collection | AbstractClass::Reversible
    );
    match ty {
        Type::Set(_) => always(
            kind != AbstractClass::Reversible
                && kind != AbstractClass::Iterator
                && kind != AbstractClass::Generator,
        ),
        Type::List(_)
        | Type::Tuple(_)
        | Type::TupleOf(_)
        | Type::Str
        | Type::Bytes
        | Type::Dict(..)
        | Type::DictView(..)
        | Type::Range => always(kind == AbstractClass::Iterable || collection),
        Type::Generator { .. }
        | Type::Iterator(_)
        | Type::TextFile
        | Type::BinaryFile(_)
        | Type::DescriptorFile(_)
            if collection =>
        {
            Outcome::Never
        }
        // A generator is all three; what an iterator holds, or an iterable, its object knows
        // (TYP-21).
        Type::Generator { .. } => Outcome::Always,
        Type::Iterator(_) if kind == AbstractClass::Generator => Outcome::Checked,
        Type::TextFile | Type::BinaryFile(_) | Type::DescriptorFile(_)
            if kind == AbstractClass::Generator =>
        {
            Outcome::Never
        }
        Type::Iterator(_) | Type::TextFile | Type::BinaryFile(_) | Type::DescriptorFile(_) => {
            Outcome::Always
        }
        Type::Iterable(_) if kind == AbstractClass::Iterable => Outcome::Always,
        Type::Iterable(_) => Outcome::Checked,
        Type::Instance(own) => {
            let inherits = |(class, _): &(crate::typed::ClassId, Vec<crate::typed::ClassId>)| {
                own.id == *class || own.ancestors.iter().any(|(each, _)| each == class)
            };
            let below = |(_, bases): &(crate::typed::ClassId, Vec<crate::typed::ClassId>)| {
                bases.contains(&own.id)
            };
            if implementers.iter().any(inherits) {
                Outcome::Always
            } else if implementers.iter().any(below) {
                // A subclass defines the methods, so the instance's class decides.
                Outcome::Checked
            } else {
                Outcome::Never
            }
        }
        _ => Outcome::Never,
    }
}

/// Returns what testing a value of type `ty`, which is not optional, against `class` gives.
fn outcome(ty: &Type, class: &TypeObjectOf) -> Outcome {
    if let TypeObjectOf::Abstract { kind, implementers } = class
        && !matches!(ty, Type::Union(_) | Type::Object)
    {
        return abstract_outcome(ty, *kind, implementers);
    }
    match (ty, class) {
        // An `object` may hold an instance of any class (TYP-23), and `None` is the one
        // instance of its class.
        (Type::Object, TypeObjectOf::Object) | (Type::NoneType, TypeObjectOf::NoneType) => {
            Outcome::Always
        }
        (Type::Object, _) => Outcome::Checked,
        // A union is an instance when each member is, and is not when no member is.
        (Type::Union(members), _) => {
            let each: Vec<Outcome> = members
                .iter()
                .map(|member| outcome(member, class))
                .collect();
            if each.iter().all(|one| matches!(one, Outcome::Always)) {
                Outcome::Always
            } else if each.iter().all(|one| matches!(one, Outcome::Never)) {
                Outcome::Never
            } else {
                Outcome::Checked
            }
        }
        (_, TypeObjectOf::NoneType) => Outcome::Never,
        (Type::Instance(own), TypeObjectOf::Instance(id, _)) => {
            let copy_of = own.generic.as_ref().is_some_and(|(root, _)| root == id);
            if own.id == *id || copy_of || own.ancestors.iter().any(|(each, _)| each == id) {
                Outcome::Always
            } else {
                // The instance's class may derive from the tested class; when the two are not
                // related, the check gives false.
                Outcome::Checked
            }
        }
        (Type::Exception(own), TypeObjectOf::Exception(tested)) => {
            if own.derives_from(tested) {
                Outcome::Always
            } else if tested.derives_from(own) {
                Outcome::Checked
            } else {
                Outcome::Never
            }
        }
        // Every value is an `object`, a `bool` is an `int` in CPython, and a named tuple is a
        // `tuple` (CLS-27).
        (_, TypeObjectOf::Object) | (Type::Bool, TypeObjectOf::Int) => Outcome::Always,
        (Type::Instance(own), TypeObjectOf::Tuple) if own.operators.named_tuple() => {
            Outcome::Always
        }
        // These hold an object whose type only the object knows.
        (Type::Iterator(_) | Type::Iterable(_) | Type::AnyModule, _) => Outcome::Checked,
        _ => match TypeObjectOf::of(ty) {
            Some(own) if own == *class => Outcome::Always,
            _ => Outcome::Never,
        },
    }
}

/// A class the second argument of `isinstance` names: one the program names, or one held in
/// a value, which is tested when the program runs (CLS-18).
enum Named {
    Class(TypeObjectOf),
    Held(TypedExpr),
}

/// Returns true for a tuple of classes held in a value, such as `kinds = (int, str)` or a
/// `tuple[type[Shape], ...]`, each item of which is tested as a held class (CLS-18, CLS-19).
fn holds_classes(ty: &Type) -> bool {
    match ty {
        Type::Tuple(items) => items.iter().all(|item| matches!(item, Type::TypeObject(_))),
        Type::TupleOf(item) => matches!(item.as_ref(), Type::TypeObject(_)),
        _ => false,
    }
}
