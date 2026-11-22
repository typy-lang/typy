//! Calls of a method of a built-in type with `*` of a value only the run knows (EXP-22,
//! ADR-0083). The receiver is evaluated first into a hidden variable, then the call is checked
//! for each number of arguments the method takes, as a call of a built-in function is, with
//! CPython 3.15's messages for any other number. The methods of a set that take any number of
//! iterables apply the method that updates a set to each in turn: a copy of the receiver for
//! `union`, `intersection` and `difference`, and the receiver itself for the others.

use typy_diagnostics::Span;
use typy_source::syntax::{Comprehension, Expr, Identifier, Keyword};

use super::BodyChecker;
use super::builtin_unpacking::Arity;
use super::method_arity::method_arity;
use crate::method::Method;
use crate::typed::{SymbolId, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `receiver.name(arguments, keywords)` of a method of a built-in type with `*` of
    /// a value only the run knows; `None` for any other call.
    pub(super) fn method_unpacked(
        &mut self,
        (receiver, name): (&Expr, &Identifier),
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        if keywords.iter().any(|keyword| keyword.name.is_none()) {
            return None;
        }
        let value = self.expression(receiver);
        let class = receiver_class(&value.ty)?;
        Method::of(&value.ty, &name.name)?;
        let (_, _, minimum, maximum, few, many) = method_arity(class, &name.name)?;
        let at = receiver.span();
        let holder = self.call_variable("receiver", value.ty.clone(), at);
        let prelude = vec![Self::stored(holder, value)];
        let callee = Expr::Attribute {
            value: Box::new(self.hidden_name(holder, at)),
            attribute: name.clone(),
            span: at,
        };
        let shown = format!("{class}.{}", name.name);
        let Some(maximum) = maximum else {
            return Some(self.set_fold((holder, name), prelude, arguments, span));
        };
        let arity = Arity::new(minimum, maximum, few, many);
        Some(self.arity_arms(
            (&callee, &shown),
            &arity,
            prelude,
            (arguments, keywords),
            span,
        ))
    }

    /// Checks `receiver.name(*iterables)` of a method of a set that takes any number of
    /// iterables, the receiver held by the hidden variable `receiver`: the method that updates
    /// a set is applied to each iterable in turn, on a copy of the receiver for a method that
    /// returns a new set.
    fn set_fold(
        &mut self,
        (receiver, name): (SymbolId, &Identifier),
        mut prelude: Vec<TypedExpr>,
        arguments: &[Expr],
        span: Span,
    ) -> TypedExpr {
        // The call takes what it unpacks, so its own errors stand (ADR-0073).
        self.unpacking_calls.push(span);
        let what = format!("the arguments of `set.{}` called with `*`", name.name);
        let (list, _) = self.one_type_values(arguments, &what, span);
        let iterables = self.call_variable("unpacked", list.ty.clone(), span);
        prelude.push(Self::stored(iterables, list));
        let (updated, update) = match name.name.as_str() {
            "union" => (None, "update"),
            "intersection" => (None, "intersection_update"),
            "difference" => (None, "difference_update"),
            other => (Some(receiver), other),
        };
        let target = updated.unwrap_or_else(|| {
            let copy = self.method_of(receiver, "copy", Vec::new(), span);
            let copy = self.expression(&copy);
            let target = self.call_variable("copy", copy.ty.clone(), span);
            prelude.push(Self::stored(target, copy));
            target
        });
        // The comprehension binds its own variable, named as no program can name one.
        let each = Expr::Name {
            id: format!("<each {}>", self.local_order.len()),
            span,
        };
        let element = self.method_of(target, update, vec![each.clone()], span);
        let applied = Expr::ListComp {
            element: Box::new(element),
            generators: vec![Comprehension {
                target: each,
                iter: self.hidden_name(iterables, span),
                ifs: Vec::new(),
                is_async: false,
                span,
            }],
            span,
        };
        prelude.push(self.expression(&applied));
        let result = if updated.is_some() {
            Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::NoneType,
                span,
            )
        } else {
            let ty = self.program.symbols[target.0 as usize].ty.clone();
            Self::typed(TypedExprKind::Local(target), ty, span)
        };
        Self::in_order(prelude, result)
    }

    /// Returns `variable.name(arguments)` of the hidden variable `variable`.
    fn method_of(&self, variable: SymbolId, name: &str, arguments: Vec<Expr>, span: Span) -> Expr {
        Expr::Call {
            function: Box::new(Expr::Attribute {
                value: Box::new(self.hidden_name(variable, span)),
                attribute: Identifier {
                    name: name.to_owned(),
                    span,
                },
                span,
            }),
            arguments,
            keywords: Vec::new(),
            span,
        }
    }
}

/// Returns the name of the class of a value of `ty` whose methods the table of arities
/// gives, or `None`.
fn receiver_class(ty: &Type) -> Option<&'static str> {
    Some(match ty {
        Type::Str => "str",
        Type::Bytes => "bytes",
        Type::List(_) => "list",
        Type::Dict(..) => "dict",
        Type::Set(_) => "set",
        Type::TupleOf(_) => "tuple",
        Type::Slice => "slice",
        Type::Generator { .. } => "generator",
        Type::TextFile | Type::DescriptorFile(None) => "TextIOWrapper",
        Type::BinaryFile(kind) | Type::DescriptorFile(Some(kind)) => kind.class_name(),
        _ => return None,
    })
}
