//! The bridge of a static method (CLS-06): a method of the static method's name that takes
//! the instance first and calls the static method with the rest. The method table holds it
//! in the static method's place, so a call through an instance reaches the method of the
//! object's own class, a static method or not, when a class derived from the instance's
//! static class defines the name again. A call through the class calls the static method.

use super::Program;
use crate::typed::{
    Argument, ClassId, FunctionId, FunctionSignature, SignatureParameter, SymbolKind, TypedExpr,
    TypedExprKind, TypedFunction, TypedStmt,
};

impl Program {
    /// Adds the bridge of each static method `class` defines.
    pub(crate) fn add_static_bridges(&mut self, class: ClassId) {
        // A copy whose type arguments hold a type parameter is checked, never run, and a
        // static method with type parameters of its own is called through its copies.
        if self.is_opaque_class(class) {
            return;
        }
        let info = &self.classes[class.0 as usize];
        let statics: Vec<(String, FunctionId)> = info
            .methods
            .iter()
            .filter(|(name, _)| {
                info.static_methods.contains(name) && !info.generic_methods.contains(name)
            })
            .cloned()
            .collect();
        for (name, method) in statics {
            let made = self.static_bridge(class, &name, method);
            self.classes[class.0 as usize]
                .static_bridges
                .push((name, made));
        }
    }

    /// Returns the bridge of the static method `method`, named `name`, of `class`.
    fn static_bridge(&mut self, class: ClassId, name: &str, method: FunctionId) -> FunctionId {
        let info = &self.classes[class.0 as usize];
        let (class_name, span) = (info.name.clone(), info.span);
        let instance = self.instance_type(class);
        let called = self.signatures[method.0 as usize].clone();
        let mut parameters = vec![SignatureParameter {
            name: "self".to_owned(),
            ty: instance,
            default: None,
            default_call: None,
            kind: typy_source::syntax::ParameterKind::Normal,
        }];
        parameters.extend(called.parameters.iter().cloned());
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
            .skip(1)
            .enumerate()
            .map(|(index, (symbol, parameter))| Argument {
                parameter: index,
                value: TypedExpr {
                    kind: TypedExprKind::Local(*symbol),
                    ty: parameter.ty.clone(),
                    span,
                },
            })
            .collect();
        let call = TypedExpr {
            kind: TypedExprKind::Call {
                function: method,
                arguments,
                dispatch: false,
            },
            ty: called.returns.clone(),
            span,
        };
        let signature = FunctionSignature {
            parameters,
            returns: called.returns,
        };
        let id = self.next_function();
        self.signatures.push(signature.clone());
        let module = self.class_modules[class.0 as usize].clone();
        self.functions.push(Some(TypedFunction {
            id,
            name: format!("<static {name}>"),
            qualified_name: format!("{class_name}.<static {name}>"),
            module,
            signature,
            parameters: symbols,
            locals: Vec::new(),
            body: vec![TypedStmt::Return {
                value: Some(call),
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
