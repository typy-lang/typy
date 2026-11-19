//! The bridges that let an instance call a class method (CLS-25, ADR-0037): each copy of a
//! class method gets a method of the class method's name that takes the instance and the
//! copy's other parameters and calls the copy with its own class, so a call through an
//! instance goes through the method table to the bridge of the instance's class.

use super::Program;
use crate::typed::{
    Argument, ClassId, FunctionId, FunctionSignature, SignatureParameter, SymbolKind, TypedExpr,
    TypedExprKind, TypedFunction, TypedStmt,
};
use crate::types::{Type, TypeObjectOf};

impl Program {
    /// Adds a bridge for each class method of `class` the class has no method of the same
    /// name for: its own method of that name hides the class method from its instances.
    pub(crate) fn add_class_method_bridges(&mut self, class: ClassId) {
        let info = &self.classes[class.0 as usize];
        // A class method with type parameters is called through its copies, which a bridge
        // cannot name (GEN-16).
        let wanted: Vec<(String, FunctionId)> = info
            .class_methods
            .iter()
            .filter(|(name, copy)| {
                info.methods.iter().all(|(each, _)| each != name) && self.generic(*copy).is_none()
            })
            .cloned()
            .collect();
        for (name, copy) in wanted {
            let bridge = self.class_method_bridge(class, &name, copy);
            self.classes[class.0 as usize].methods.push((name, bridge));
        }
    }

    /// Returns the bridge of the copy `copy` of the class method `name` of `class`.
    fn class_method_bridge(&mut self, class: ClassId, name: &str, copy: FunctionId) -> FunctionId {
        let id = self.next_function();
        let info = &self.classes[class.0 as usize];
        let (class_name, span) = (info.name.clone(), info.span);
        let copied = self.signatures[copy.0 as usize].clone();
        let instance = self.instance_type(class);
        let of = match self.exception(class) {
            Some(exception) => TypeObjectOf::Exception(exception.clone()),
            None => TypeObjectOf::Instance(class, class_name.clone()),
        };
        let mut parameters = vec![SignatureParameter {
            name: "self".to_owned(),
            ty: instance.clone(),
            default: None,
            default_call: None,
            kind: typy_source::syntax::ParameterKind::Normal,
        }];
        parameters.extend(copied.parameters.iter().skip(1).cloned());
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
        let typed = |kind: TypedExprKind, ty: Type| TypedExpr { kind, ty, span };
        let mut arguments = vec![Argument {
            parameter: 0,
            value: typed(
                TypedExprKind::TypeObject(Box::new(of.clone())),
                TypeObjectOf::type_of(Some(of)),
            ),
        }];
        arguments.extend(symbols.iter().zip(&parameters).enumerate().skip(1).map(
            |(index, (symbol, parameter))| Argument {
                parameter: index,
                value: typed(TypedExprKind::Local(*symbol), parameter.ty.clone()),
            },
        ));
        let call = typed(
            TypedExprKind::Call {
                function: copy,
                arguments,
                dispatch: false,
            },
            copied.returns.clone(),
        );
        let signature = FunctionSignature {
            parameters,
            returns: copied.returns,
        };
        self.signatures.push(signature.clone());
        let module = self.class_modules[class.0 as usize].clone();
        self.functions.push(Some(TypedFunction {
            id,
            name: name.to_owned(),
            qualified_name: format!("{class_name}.{name}"),
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
