//! The `<truth>` method of a class whose truth comes from `__len__` (CLS-24): it returns
//! `len(self) != 0`, the length checked as `len()` checks it, and the `truth` slot of the
//! class's type object calls it, so `any`, `all` and a truth test through the runtime test such
//! an instance as CPython does.

use super::Program;
use crate::typed::{
    Argument, ClassId, FunctionSignature, SignatureParameter, SymbolKind, TypedConstant, TypedExpr,
    TypedExprKind, TypedFunction, TypedStmt,
};
use crate::types::Type;
use typy_source::syntax::CompareOperator;

/// The name of the method, which no Python name can be.
pub const TRUTH_BRIDGE: &str = "<truth>";

impl Program {
    /// Adds `<truth>` to `class` when it defines `__len__` and has no `__bool__`, which CPython
    /// would ask first.
    pub(crate) fn add_truth_bridge(&mut self, class: ClassId) {
        let info = &self.classes[class.0 as usize];
        let Some(&(_, length)) = info.methods.iter().find(|(name, _)| name == "__len__") else {
            return;
        };
        if self.method(class, "__bool__").is_some() {
            return;
        }
        let (class_name, span) = (info.name.clone(), info.span);
        let instance = self.instance_type(class);
        let symbol = self.add_symbol("self", SymbolKind::Parameter, instance.clone(), span);
        let typed = |kind: TypedExprKind, ty: Type| TypedExpr { kind, ty, span };
        let call = typed(
            TypedExprKind::Call {
                function: length,
                arguments: vec![Argument {
                    passed: crate::typed::Passed::Position,
                    parameter: 0,
                    value: typed(TypedExprKind::Local(symbol), instance.clone()),
                }],
                dispatch: true,
            },
            Type::Int,
        );
        let compared = typed(
            TypedExprKind::Compare {
                left: Box::new(typed(
                    TypedExprKind::CheckedLength(Box::new(call)),
                    Type::Int,
                )),
                operators: vec![CompareOperator::NotEq],
                comparators: vec![typed(
                    TypedExprKind::Constant(TypedConstant::Int("0".to_owned())),
                    Type::Int,
                )],
            },
            Type::Bool,
        );
        let signature = FunctionSignature {
            parameters: vec![SignatureParameter {
                name: "self".to_owned(),
                ty: instance,
                default: None,
                default_call: None,
                kind: typy_source::syntax::ParameterKind::Normal,
            }],
            returns: Type::Bool,
        };
        let id = self.next_function();
        self.signatures.push(signature.clone());
        let module = self.class_modules[class.0 as usize].clone();
        self.functions.push(Some(TypedFunction {
            id,
            name: TRUTH_BRIDGE.to_owned(),
            qualified_name: format!("{class_name}.{TRUTH_BRIDGE}"),
            module,
            signature,
            parameters: vec![symbol],
            locals: Vec::new(),
            body: vec![TypedStmt::Return {
                value: Some(compared),
                span,
            }],
            span,
            generic: false,
            named_targets: Vec::new(),
            generator: None,
        }));
        self.classes[class.0 as usize]
            .methods
            .push((TRUTH_BRIDGE.to_owned(), id));
    }
}
