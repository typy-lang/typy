//! The `<equal>` method of a class whose `__eq__` may return `NotImplemented` (CLS-21,
//! ADR-0038): the `equal` slot of the class's type object, which lists, sets and dicts call,
//! calls it for `self == other`, which gives way to the other object's `__eq__` and then
//! compares identities, as CPython does.

use super::Program;
use crate::typed::{
    ClassId, FunctionSignature, OperatorCall, OperatorKind, SignatureParameter, SymbolKind,
    TypedExpr, TypedExprKind, TypedFunction, TypedStmt,
};
use crate::types::Type;

/// The name of the method, which no Python name can be.
pub const EQUAL_BRIDGE: &str = "<equal>";

impl Program {
    /// Adds `<equal>` to `class` when the class's own `__eq__` may give way.
    pub(crate) fn add_equal_bridge(&mut self, class: ClassId) {
        let info = &self.classes[class.0 as usize];
        let Some(&(_, equal)) = info.methods.iter().find(|(name, _)| name == "__eq__") else {
            return;
        };
        let (class_name, span) = (info.name.clone(), info.span);
        let signature = self.signatures[equal.0 as usize].clone();
        if !signature.returns.may_give_way() {
            return;
        }
        // The slot calls the method for an object of the class `__eq__` takes, as it calls
        // `__eq__` itself (ADR-0026).
        let takes = match signature.parameters.get(1).map(|parameter| &parameter.ty) {
            Some(Type::Instance(takes)) => takes.id,
            Some(Type::Optional(inner)) => match inner.as_ref() {
                Type::Instance(takes) => takes.id,
                _ => class,
            },
            _ => class,
        };
        let instance = self.instance_type(class);
        let other = self.instance_type(takes);
        let reflected = self.method(takes, "__eq__").filter(|method| {
            self.signatures[method.0 as usize]
                .parameters
                .get(1)
                .is_some_and(|parameter| instance.is_assignable_to(&parameter.ty))
        });
        let parameters = [("self", instance), ("other", other)];
        let symbols: Vec<_> = parameters
            .iter()
            .map(|(name, ty)| self.add_symbol(name, SymbolKind::Parameter, ty.clone(), span))
            .collect();
        let read = |position: usize| TypedExpr {
            kind: TypedExprKind::Local(symbols[position]),
            ty: parameters[position].1.clone(),
            span,
        };
        let compared = TypedExpr {
            kind: TypedExprKind::OperatorCall(Box::new(OperatorCall {
                left: read(0),
                right: read(1),
                forward: Some(equal),
                reflected,
                left_reflected: Some(equal),
                negated: false,
                in_place: None,
                kind: OperatorKind::Equality { different: false },
            })),
            ty: Type::Bool,
            span,
        };
        let signature = FunctionSignature {
            parameters: parameters
                .iter()
                .map(|(name, ty)| SignatureParameter {
                    name: (*name).to_owned(),
                    ty: ty.clone(),
                    default: None,
                    default_call: None,
                })
                .collect(),
            returns: Type::Bool,
        };
        let id = self.next_function();
        self.signatures.push(signature.clone());
        let module = self.class_modules[class.0 as usize].clone();
        self.functions.push(Some(TypedFunction {
            id,
            name: EQUAL_BRIDGE.to_owned(),
            qualified_name: format!("{class_name}.{EQUAL_BRIDGE}"),
            module,
            signature,
            parameters: symbols,
            locals: Vec::new(),
            body: vec![TypedStmt::Return {
                value: Some(compared),
                span,
            }],
            span,
            generic: false,
            named_targets: Vec::new(),
        }));
        self.classes[class.0 as usize]
            .methods
            .push((EQUAL_BRIDGE.to_owned(), id));
    }
}
