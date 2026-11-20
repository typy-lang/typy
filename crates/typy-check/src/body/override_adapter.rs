//! The adapter of an override whose parameters or result are held otherwise than its base
//! method's (CLS-04): a method with the base's signature that converts each argument to the
//! override's parameter, calls the override through the method table, and converts its
//! result to the base's. A call through the base reaches the override through it, and a call
//! through the override's class reaches the override itself.

use super::{BodyChecker, Program};
use crate::typed::{
    Argument, ClassId, FunctionId, FunctionSignature, SignatureParameter, SymbolKind, TypedExpr,
    TypedExprKind, TypedFunction, TypedStmt,
};
use crate::types::Type;

impl Program {
    /// Adds the adapter of each method of `class` that overrides one held otherwise.
    pub(crate) fn add_override_adapters(&mut self, class: ClassId) {
        let adapted = self.classes[class.0 as usize].adapted.clone();
        for (name, base, own) in adapted {
            let made = self.override_adapter(class, &name, base, own);
            self.classes[class.0 as usize].adapters.push((name, made));
        }
    }

    /// Returns the adapter of `own`, the method `name` of `class`, to `base`'s signature.
    fn override_adapter(
        &mut self,
        class: ClassId,
        name: &str,
        base: FunctionId,
        own: FunctionId,
    ) -> FunctionId {
        let info = &self.classes[class.0 as usize];
        let (class_name, span) = (info.name.clone(), info.span);
        let instance = self.instance_type(class);
        let mut parameters = self.signatures[base.0 as usize].parameters.clone();
        let returns = self.signatures[base.0 as usize].returns.clone();
        let own_signature = self.signatures[own.0 as usize].clone();
        if let Some(first) = parameters.first_mut() {
            first.ty = instance;
        }
        let symbols: Vec<_> = parameters
            .iter()
            .map(|parameter| {
                self.add_symbol(
                    &parameter.name,
                    SymbolKind::Parameter,
                    parameter.ty.clone(),
                    span,
                )
            })
            .collect();
        let arguments = symbols
            .iter()
            .zip(&parameters)
            .zip(&own_signature.parameters)
            .enumerate()
            .map(|(index, ((symbol, given), wanted))| {
                let read = TypedExpr {
                    kind: TypedExprKind::Local(*symbol),
                    ty: given.ty.clone(),
                    span,
                };
                Argument {
                    keyword: None,
                    parameter: index,
                    value: widened(read, &wanted.ty),
                }
            })
            .collect();
        let call = TypedExpr {
            kind: TypedExprKind::Call {
                function: own,
                arguments,
                dispatch: true,
            },
            ty: own_signature.returns.clone(),
            span,
        };
        let signature = FunctionSignature {
            parameters: parameters
                .iter()
                .map(|parameter| SignatureParameter {
                    default: parameter.default.clone(),
                    ..parameter.clone()
                })
                .collect(),
            returns: returns.clone(),
        };
        let id = self.next_function();
        self.signatures.push(signature.clone());
        let module = self.class_modules[class.0 as usize].clone();
        self.functions.push(Some(TypedFunction {
            id,
            name: format!("<adapter {name}>"),
            qualified_name: format!("{class_name}.<adapter {name}>"),
            module,
            signature,
            parameters: symbols,
            locals: Vec::new(),
            body: vec![TypedStmt::Return {
                value: Some(widened(call, &returns)),
                span,
            }],
            span,
            generic: false,
            named_targets: Vec::new(),
            generator: None,
        }));
        id
    }
}

/// Returns `value` held as a value of `target`, a type that accepts it: wrapped into an
/// optional, or widened to a union or a base class.
fn widened(value: TypedExpr, target: &Type) -> TypedExpr {
    if value.ty == *target {
        return value;
    }
    if matches!(target, Type::Optional(_)) && !matches!(value.ty, Type::Optional(_)) {
        return BodyChecker::converted(value, Some(target));
    }
    let span = value.span;
    TypedExpr {
        kind: TypedExprKind::Upcast(Box::new(value)),
        ty: target.clone(),
        span,
    }
}
