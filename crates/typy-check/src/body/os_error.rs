//! `OSError` and the classes derived from it, which CPython builds from arguments of any type:
//! two to five are read as `errno`, `strerror`, `filename`, `winerror` and `filename2`, and
//! the attributes are those items, or `None` (EXC-14, EXC-16, ADR-0035).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{CompareOperator, Identifier};

use super::BodyChecker;
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::{EachMember, SymbolKind, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::{Type, TypeObjectOf};

/// An attribute of an `OSError` that its arguments give.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    /// `errno`, the first of two to five arguments.
    Errno,
    /// `strerror`, the second.
    Strerror,
    /// `filename`, the third of three to five.
    Filename,
    /// `filename2`, the fifth of five, when the third is not `None`.
    Filename2,
}

impl BodyChecker<'_> {
    /// Refuses a number as the third of three to five arguments of `OSError` or
    /// `BlockingIOError`: an exception whose class is exactly `BlockingIOError` when the
    /// program runs reads it as `characters_written`, and `OSError` becomes that class for
    /// `errno` 11 (EXC-18).
    pub(super) fn os_error_arguments(
        &mut self,
        class: &ExceptionClass,
        values: &[TypedExpr],
        span: Span,
    ) -> Option<TypedExpr> {
        let may_block = matches!(
            class,
            ExceptionClass::Builtin(BuiltinException::OSError | BuiltinException::BlockingIOError)
        );
        if !may_block || !(3..=5).contains(&values.len()) || !holds_number(&values[2].ty) {
            return None;
        }
        Some(
            self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    values[2].span,
                    "a number as the third argument of `OSError` or `BlockingIOError`, which \
                     a `BlockingIOError` reads as `characters_written`, is planned for TyPy 1.0 \
                     (EXC-18)",
                )
                .with_rule(RuleId::EXC_18)
                .with_help("give the file name as a `str`"),
                span,
            ),
        )
    }

    /// Returns `errno`, `strerror`, `filename` or `filename2` of an `OSError`, when `object`
    /// is one and the name is one of them, and its class declares no attribute of that name:
    /// for each layout of the arguments the creations of its class give, the item the
    /// attribute names, or `None` (ADR-0035). The exception is evaluated once.
    pub(super) fn os_error_field(
        &mut self,
        object: &TypedExpr,
        attribute: &Identifier,
        span: Span,
    ) -> Option<TypedExpr> {
        let Type::Exception(class) = &object.ty else {
            return None;
        };
        if !class.builtin().is_os_error() {
            return None;
        }
        if let ExceptionClass::Class(own) = class
            && self.program.classes[own.id.0 as usize]
                .attribute(&attribute.name)
                .is_some()
        {
            return None;
        }
        let field = match attribute.name.as_str() {
            "errno" => Field::Errno,
            "strerror" => Field::Strerror,
            "filename" => Field::Filename,
            "filename2" => Field::Filename2,
            _ => return None,
        };
        let raw = self.program.raw_arguments_type(&class.clone(), span);
        let arguments = |exception: TypedExpr, ty: Type| {
            Self::typed(
                TypedExprKind::ExceptionArguments {
                    exception: Box::new(exception),
                    raw: true,
                },
                ty,
                span,
            )
        };
        if raw.has_error() {
            return Some(arguments(object.clone(), Type::Error));
        }
        let members = raw.members().to_vec();
        let ty = Type::union(
            members
                .iter()
                .map(|member| field_type(member, field))
                .collect(),
        );
        if let [member] = members.as_slice() {
            let read = arguments(object.clone(), member.clone());
            return Some(Self::converted(
                projected(read, member, field, span),
                Some(&ty),
            ));
        }
        // The value decides the member, and each member's arm reads the exception again, so a
        // value that is not a variable is held in a hidden one first.
        let (first, again) = match object.kind {
            TypedExprKind::Local(_) | TypedExprKind::ModuleVariable(_) => {
                (object.clone(), object.clone())
            }
            _ => self.held_exception(object, span),
        };
        let arms = members
            .iter()
            .map(|member| {
                let read = arguments(again.clone(), member.clone());
                (
                    crate::typed::MemberTest::Class(TypeObjectOf::Tuple),
                    Self::converted(projected(read, member, field, span), Some(&ty)),
                )
            })
            .collect();
        Some(Self::typed(
            TypedExprKind::EachMember(Box::new(EachMember {
                value: arguments(first, raw),
                arms,
            })),
            ty,
            span,
        ))
    }

    /// Returns `object` stored in a new hidden variable, and a read of the variable.
    fn held_exception(&mut self, object: &TypedExpr, span: Span) -> (TypedExpr, TypedExpr) {
        let name = format!("<exception {}>", self.local_order.len());
        let holder = self
            .program
            .add_symbol(&name, SymbolKind::Local, object.ty.clone(), span);
        self.locals.insert(name, holder);
        self.local_order.push(holder);
        self.types.insert(holder, object.ty.clone());
        let stored = Self::typed(
            TypedExprKind::Named {
                target: holder,
                value: Box::new(object.clone()),
            },
            object.ty.clone(),
            span,
        );
        let read = Self::typed(TypedExprKind::Local(holder), object.ty.clone(), span);
        (stored, read)
    }
}

/// Returns true when a value of the type may be a number: an `int`, a `float` or a `bool`.
fn holds_number(ty: &Type) -> bool {
    let inner = match ty {
        Type::Optional(inner) => inner.as_ref(),
        other => other,
    };
    inner
        .members()
        .iter()
        .any(|member| matches!(member, Type::Int | Type::Float | Type::Bool))
}

/// Returns the position of the item `field` reads from arguments of `length` items, when
/// it reads one.
const fn position(field: Field, length: usize) -> Option<usize> {
    match (field, length) {
        (Field::Errno, 2..=5) => Some(0),
        (Field::Strerror, 2..=5) => Some(1),
        (Field::Filename, 3..=5) => Some(2),
        (Field::Filename2, 5) => Some(4),
        _ => None,
    }
}

/// Returns the type `field` has for arguments of the tuple type `member`.
fn field_type(member: &Type, field: Field) -> Type {
    let Type::Tuple(items) = member else {
        return Type::NoneType;
    };
    match position(field, items.len()) {
        // `filename2` is set only when `filename` is not `None`.
        Some(4) => match &items[2] {
            Type::NoneType => Type::NoneType,
            Type::Optional(_) => Type::union(vec![items[4].clone(), Type::NoneType]),
            _ => items[4].clone(),
        },
        Some(index) => items[index].clone(),
        None => Type::NoneType,
    }
}

/// Returns `field` of the arguments `read` gives, of the tuple type `member`.
fn projected(read: TypedExpr, member: &Type, field: Field, span: Span) -> TypedExpr {
    let none = || {
        BodyChecker::typed(
            TypedExprKind::Constant(TypedConstant::None),
            Type::NoneType,
            span,
        )
    };
    let Type::Tuple(items) = member else {
        return none();
    };
    let item = |read: TypedExpr, index: usize| {
        BodyChecker::typed(
            TypedExprKind::TupleItem {
                tuple: Box::new(read),
                index,
            },
            items[index].clone(),
            span,
        )
    };
    match position(field, items.len()) {
        Some(4) if items[2] == Type::NoneType => none(),
        // `read.filename2 if read.filename is not None else None`.
        Some(4) if matches!(items[2], Type::Optional(_)) => {
            let ty = field_type(member, field);
            let test = BodyChecker::typed(
                TypedExprKind::Compare {
                    left: Box::new(item(read.clone(), 2)),
                    operators: Box::new([CompareOperator::IsNot]),
                    comparators: vec![none()],
                    narrowing: None,
                },
                Type::Bool,
                span,
            );
            BodyChecker::typed(
                TypedExprKind::IfExp {
                    test: Box::new(test),
                    body: Box::new(BodyChecker::converted(item(read, 4), Some(&ty))),
                    orelse: Box::new(BodyChecker::converted(none(), Some(&ty))),
                },
                ty,
                span,
            )
        }
        Some(index) => item(read, index),
        None => none(),
    }
}
