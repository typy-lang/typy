//! A method read through a protocol value as a value (FUN-09, ADR-0079): a bound method of the
//! method of the object's class, chosen among the classes that satisfy the protocol once every
//! module is checked. A class whose method takes or gives a type held otherwise than the
//! protocol's method gets an adapter of the protocol's signature, which takes the method's
//! name and identity, as an override's adapter does; a base's method read through an
//! instance gets one of the base's signature the same way.

use typy_diagnostics::{Diagnostics, Span};
use typy_source::syntax::Identifier;

use super::callable_adapter::Expected;
use super::{BodyChecker, Program};
use crate::typed::{
    Argument, ClassId, FunctionId, SymbolId, SymbolKind, TypedExpr, TypedExprKind, TypedFunction,
    TypedStmt,
};
use crate::types::Type;

/// A method read through a protocol value where a `Callable` takes it through an adapter: the
/// protocol, the method's name, the `Callable`, the adapter of the protocol's dispatcher that
/// stands when no class satisfies the protocol, and where the read is.
#[derive(Clone, Debug)]
pub(crate) struct AdaptedRead {
    protocol: ClassId,
    name: String,
    expected: Expected,
    dispatched: FunctionId,
    span: Span,
}

impl BodyChecker<'_> {
    /// Returns `object.name` of the method `name` of the protocol `protocol`, read through a
    /// protocol value: a bound method of the method of the object's class, of the `Callable`
    /// type of the protocol's method's parameters after `self`, or of the `Callable` its
    /// context expects through an adapter (FUN-19).
    pub(super) fn protocol_method_value(
        &mut self,
        object: TypedExpr,
        (protocol, method): (ClassId, FunctionId),
        name: &Identifier,
        span: Span,
    ) -> TypedExpr {
        if let Some(expected) = self.adapted_callable(method, 1, span) {
            return self.adapted_protocol_method(object, protocol, (&name.name, span), expected);
        }
        let signature = &self.program.signatures[method.0 as usize];
        let ty = Type::Callable {
            parameters: signature
                .parameters
                .iter()
                .skip(1)
                .map(|parameter| parameter.ty.clone())
                .collect(),
            returns: Box::new(signature.returns.clone()),
        };
        self.program
            .protocol_values
            .method_values
            .insert((protocol, name.name.clone()));
        Self::typed(
            TypedExprKind::ProtocolMethod {
                object: Box::new(object),
                protocol,
                name: name.name.clone(),
            },
            ty,
            span,
        )
    }
}

impl BodyChecker<'_> {
    /// Returns `object.name` of the method `name` of `protocol` as a value of the `Callable`
    /// `expected`: a bound method of the adapter of the method of the object's class, chosen
    /// once every module is checked. The adapter of the protocol's dispatcher checks the call
    /// now, and stands when no class satisfies the protocol.
    fn adapted_protocol_method(
        &mut self,
        object: TypedExpr,
        protocol: ClassId,
        (name, span): (&str, Span),
        expected: Expected,
    ) -> TypedExpr {
        let dispatcher = self
            .program
            .dispatcher(protocol, name)
            .unwrap_or(FunctionId(0));
        let callable = (expected.0.as_slice(), expected.1.as_ref());
        let adapter =
            match self.method_adapter((protocol, dispatcher), (name, true), callable, span) {
                Ok(adapter) => adapter,
                Err(error) => return error,
            };
        let ty = self.adapted_type(&expected.0, Some(adapter));
        let key = format!("{name} <adapter {}>", adapter.0);
        self.program.protocol_values.adapted_reads.insert(
            key.clone(),
            AdaptedRead {
                protocol,
                name: name.to_owned(),
                expected,
                dispatched: adapter,
                span,
            },
        );
        Self::typed(
            TypedExprKind::ProtocolMethod {
                object: Box::new(object),
                protocol,
                name: key,
            },
            ty,
            span,
        )
    }
}

impl Program {
    /// Chooses, for each method read through a protocol value, the method of each class that
    /// satisfies the protocol, the least derived first, or its adapter to the protocol's
    /// signature; the protocol's own method stands when no class satisfies it, since no value
    /// of it is then made. A read a `Callable` takes through an adapter binds the adapter of
    /// each class's method, and reports where one does not fit (FUN-19).
    pub(crate) fn write_protocol_method_values(&mut self, diagnostics: &mut Diagnostics) {
        self.write_adapted_reads(diagnostics);
        let reads: Vec<(ClassId, String)> =
            self.protocol_values.method_values.iter().cloned().collect();
        for (protocol, name) in reads {
            let Some(wanted) = self.classes[protocol.0 as usize].method(&name) else {
                continue;
            };
            let mut classes = self.satisfying_classes(protocol);
            classes.reverse();
            let mut chosen = Vec::new();
            for class in classes {
                let Some(own) = self.method(class, &name) else {
                    continue;
                };
                let function = if self.same_after_self(own, wanted) {
                    own
                } else {
                    self.value_adapter(class, &name, (own, wanted))
                };
                chosen.push((class, function));
            }
            if chosen.is_empty() {
                chosen.push((protocol, wanted));
            }
            self.protocol_values
                .bound_methods
                .insert((protocol, name), chosen);
        }
    }

    /// Chooses, for each read of `adapted_reads`, the adapter of the method of each class that
    /// satisfies the protocol, or the dispatcher's when none does. A class satisfies the
    /// protocol only with a method of the protocol's parameters and defaults, so its adapter
    /// fits the `Callable` the protocol's did.
    fn write_adapted_reads(&mut self, diagnostics: &mut Diagnostics) {
        let reads = std::mem::take(&mut self.protocol_values.adapted_reads);
        for (key, read) in &reads {
            let mut classes = self.satisfying_classes(read.protocol);
            classes.reverse();
            let mut chosen = Vec::new();
            let methods: Vec<(ClassId, FunctionId)> = classes
                .into_iter()
                .filter_map(|class| Some((class, self.method(class, &read.name)?)))
                .collect();
            for method in methods {
                let callable = (read.expected.0.as_slice(), read.expected.1.as_ref());
                let mut checker = BodyChecker::new(self, diagnostics, None);
                let bound = (read.name.as_str(), true);
                let adapter = checker.method_adapter(method, bound, callable, read.span);
                chosen.extend(adapter.ok().map(|adapter| (method.0, adapter)));
            }
            if chosen.is_empty() {
                chosen.push((read.protocol, read.dispatched));
            }
            self.protocol_values
                .bound_methods
                .insert((read.protocol, key.clone()), chosen);
        }
        self.protocol_values.adapted_reads = reads;
    }

    /// Returns true when two methods take and give the same types after the instance.
    fn same_after_self(&self, own: FunctionId, wanted: FunctionId) -> bool {
        let (own, wanted) = (
            &self.signatures[own.0 as usize],
            &self.signatures[wanted.0 as usize],
        );
        own.returns == wanted.returns
            && own.parameters.len() == wanted.parameters.len()
            && own
                .parameters
                .iter()
                .zip(&wanted.parameters)
                .skip(1)
                .all(|(own, wanted)| own.ty == wanted.ty)
    }

    /// Returns the adapter of `own`, the method `name` of `class`, to the signature of
    /// `wanted`, a protocol's method or a base's that a value reads: it takes an instance of
    /// `class` and `wanted`'s arguments, converts each to `own`'s, calls `own` through the
    /// method table, and converts the result to `wanted`'s. It is made once for each pair.
    pub(super) fn value_adapter(
        &mut self,
        class: ClassId,
        name: &str,
        (own, wanted): (FunctionId, FunctionId),
    ) -> FunctionId {
        if let Some(adapter) = self.protocol_values.adapters.get(&(own, wanted)) {
            return *adapter;
        }
        let info = &self.classes[class.0 as usize];
        let (class_name, span) = (info.name.clone(), info.span);
        let instance = self.instance_type(class);
        let own_signature = self.signatures[own.0 as usize].clone();
        let mut signature = self.signatures[wanted.0 as usize].clone();
        if let Some(first) = signature.parameters.first_mut() {
            first.ty = instance;
            first.default = None;
        }
        let symbols: Vec<SymbolId> = signature
            .parameters
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
        let arguments = signature
            .parameters
            .iter()
            .zip(&symbols)
            .zip(&own_signature.parameters)
            .enumerate()
            .map(|(index, ((given, symbol), parameter))| Argument {
                passed: crate::typed::Passed::Position,
                parameter: index,
                value: BodyChecker::converted(
                    TypedExpr {
                        kind: TypedExprKind::Local(*symbol),
                        ty: given.ty.clone(),
                        span,
                    },
                    Some(&parameter.ty),
                ),
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
        let id = self.next_function();
        let module = self.class_modules[class.0 as usize].clone();
        let returns = signature.returns.clone();
        self.signatures.push(signature.clone());
        self.functions.push(Some(TypedFunction {
            id,
            name: name.to_owned(),
            qualified_name: format!("{class_name}.{name}"),
            module,
            signature,
            parameters: symbols,
            locals: Vec::new(),
            body: vec![TypedStmt::Return {
                value: Some(BodyChecker::converted(call, Some(&returns))),
                span,
            }],
            span,
            generic: false,
            named_targets: Vec::new(),
            generator: None,
        }));
        self.protocol_values.adapters.insert((own, wanted), id);
        id
    }
}
