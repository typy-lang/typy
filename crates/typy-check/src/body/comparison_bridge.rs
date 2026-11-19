//! The comparison bridges of a class whose comparison methods may return `NotImplemented`
//! (CLS-21, CLS-22, ADR-0038, ADR-0039): `<equal>` returns `self == other`, and `<less>`,
//! `<less_equal>`, `<greater>` and `<greater_equal>` return `self < other` and the other
//! orderings, each through the steps an operator takes, which give way to the other object's
//! method and then compare identities or raise `TypeError`. The `equal` and `order` slots of
//! the class's type object, which lists, sets, dicts and sorts call, call them.

use super::Program;
use crate::typed::{
    ClassId, FunctionId, FunctionSignature, OperatorCall, OperatorKind, SignatureParameter,
    SymbolKind, TypedExpr, TypedExprKind, TypedFunction, TypedStmt,
};
use crate::types::Type;

/// The name of the bridge of `__eq__`, which no Python name can be.
pub const EQUAL_BRIDGE: &str = "<equal>";

/// Each comparison method, its reflected method, its operator and the name of its bridge, in
/// the order of the `order` slot's comparisons after `__eq__`.
pub const COMPARISON_BRIDGES: [(&str, &str, OperatorKind, &str); 5] = [
    (
        "__eq__",
        "__eq__",
        OperatorKind::Equality { different: false },
        EQUAL_BRIDGE,
    ),
    ("__lt__", "__gt__", OperatorKind::Ordering("<"), "<less>"),
    (
        "__le__",
        "__ge__",
        OperatorKind::Ordering("<="),
        "<less_equal>",
    ),
    ("__gt__", "__lt__", OperatorKind::Ordering(">"), "<greater>"),
    (
        "__ge__",
        "__le__",
        OperatorKind::Ordering(">="),
        "<greater_equal>",
    ),
];

impl Program {
    /// Adds to `class` the bridge of each comparison method of its own, which a slot calls
    /// when the method may give way or the class is one of a chain of bases, where the
    /// operands may be of different classes (CLS-21, CLS-22).
    pub(crate) fn add_comparison_bridges(&mut self, class: ClassId) {
        for (name, reflected, kind, bridge) in COMPARISON_BRIDGES {
            let own = self.classes[class.0 as usize]
                .methods
                .iter()
                .find(|(each, _)| each == name)
                .map(|(_, method)| *method);
            if let Some(method) = own {
                self.add_comparison_bridge(class, method, (reflected, kind, bridge));
            }
        }
    }

    /// Adds the bridge `bridge` of `class`'s comparison method `method`, which returns
    /// `self op other` for its operator `kind`, with the reflected method `reflected`.
    fn add_comparison_bridge(
        &mut self,
        class: ClassId,
        method: FunctionId,
        (reflected, kind, bridge): (&str, OperatorKind, &str),
    ) {
        let info = &self.classes[class.0 as usize];
        let (class_name, span) = (info.name.clone(), info.span);
        let signature = self.signatures[method.0 as usize].clone();
        // The slot calls the method for an object of the class it takes, as it calls the
        // comparison method itself (ADR-0026).
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
        let left_reflected = self.method(class, reflected);
        let reflected = self.method(takes, reflected).filter(|method| {
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
                forward: Some(method),
                reflected,
                left_reflected,
                negated: false,
                in_place: None,
                reflected_below: Vec::new(),
                kind,
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
                    kind: typy_source::syntax::ParameterKind::Normal,
                })
                .collect(),
            returns: Type::Bool,
        };
        let id = self.next_function();
        self.signatures.push(signature.clone());
        let module = self.class_modules[class.0 as usize].clone();
        self.functions.push(Some(TypedFunction {
            id,
            name: bridge.to_owned(),
            qualified_name: format!("{class_name}.{bridge}"),
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
            .push((bridge.to_owned(), id));
    }
}
