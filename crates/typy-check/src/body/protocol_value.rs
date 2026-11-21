//! Protocol values: a protocol as the type of a variable, an attribute, a result or a container
//! item holds an instance of a class that satisfies it, as an object; a member call or an
//! attribute read on one calls a dispatcher, whose body is written once every module is checked
//! (GEN-10, ADR-0041).

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::Identifier;

use super::{BodyChecker, Program};
use crate::typed::{
    Argument, ClassId, EachMember, FunctionId, FunctionSignature, MemberTest, SignatureParameter,
    SymbolId, SymbolKind, TypedExpr, TypedExprKind, TypedFunction, TypedStmt,
};
use crate::types::{Type, TypeObjectOf};

/// What the program knows of protocol values.
#[derive(Clone, Debug, Default)]
pub(crate) struct ProtocolValues {
    /// Each pair of a class and a protocol it satisfies.
    pub(crate) fits: BTreeSet<(ClassId, ClassId)>,
    /// What the instances of each iterator or iterable class are (TYP-20).
    pub(crate) iterations: std::collections::BTreeMap<ClassId, crate::iterator_fits::Iteration>,
    /// The dispatcher of each member read through a protocol value, by protocol and name.
    pub(crate) dispatchers: BTreeMap<(ClassId, String), FunctionId>,
    /// Each method read through a protocol value as a value, by protocol and name (FUN-09).
    pub(crate) method_values: BTreeSet<(ClassId, String)>,
    /// The method of each class such a read binds, by protocol and name, the least derived
    /// class first, once every module is checked.
    pub(crate) bound_methods: BTreeMap<(ClassId, String), Vec<(ClassId, FunctionId)>>,
    /// The adapter of each class's method to the signature of a method a value reads, a
    /// protocol's or a base's, by the method and that signature's method (FUN-09).
    pub(crate) adapters: BTreeMap<(FunctionId, FunctionId), FunctionId>,
}

impl Program {
    /// Notes each class that satisfies each protocol of the program, among `classes`, or among
    /// every class when `classes` is `None`. A pair whose class's attribute types are not all
    /// known is looked at again the next time.
    pub(crate) fn note_protocol_fits(&mut self, classes: Option<&[ClassId]>) {
        let protocols: Vec<ClassId> = self
            .classes
            .iter()
            .map(|info| info.id)
            .filter(|class| self.is_protocol(*class))
            .collect();
        if protocols.is_empty() {
            return;
        }
        let candidates: Vec<ClassId> = match classes {
            Some(classes) => classes.to_vec(),
            None => self.classes.iter().map(|info| info.id).collect(),
        };
        for class in candidates {
            if self.is_opaque_class(class) || self.exception(class).is_some() {
                continue;
            }
            for protocol in &protocols {
                let pair = (class, *protocol);
                if self.protocol_values.fits.contains(&pair) {
                    continue;
                }
                if self.class_unsatisfied(class, *protocol).is_none() {
                    self.protocol_values.fits.insert(pair);
                    crate::protocol_fits::note(class, *protocol);
                }
            }
        }
    }
}

/// What a dispatcher reaches through a protocol value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Member {
    /// A method, called with the protocol's method's arguments.
    Method,
    /// An attribute, read.
    Attribute,
}

impl Program {
    /// Makes the dispatcher of each method and attribute of the protocol `protocol`, whose
    /// bodies are written once every module is checked; a lookup of a method of the protocol
    /// gives its dispatcher, so every call through a protocol value reaches the object's
    /// class.
    pub(crate) fn make_protocol_dispatchers(&mut self, protocol: ClassId) {
        let info = &self.classes[protocol.0 as usize];
        let methods: Vec<String> = info.methods.iter().map(|(name, _)| name.clone()).collect();
        let attributes: Vec<String> = info
            .attributes
            .iter()
            .map(|attribute| attribute.name.clone())
            .collect();
        for name in methods {
            self.protocol_dispatcher(protocol, &name, Member::Method);
        }
        for name in attributes {
            self.protocol_dispatcher(protocol, &name, Member::Attribute);
        }
    }

    /// Returns the dispatcher of the member `name` of the protocol `class`, if it is one.
    pub(crate) fn dispatcher(&self, class: ClassId, name: &str) -> Option<FunctionId> {
        self.protocol_values
            .dispatchers
            .get(&(class, name.to_owned()))
            .copied()
    }

    /// Makes the dispatcher of the member `name` of `protocol`: its signature is the
    /// protocol's method's, or for an attribute a getter's.
    fn protocol_dispatcher(&mut self, protocol: ClassId, name: &str, member: Member) {
        let instance = self.instance_type(protocol);
        let signature = match member {
            Member::Method => {
                let info = &self.classes[protocol.0 as usize];
                let method = info.method(name).unwrap_or(FunctionId(0));
                self.signatures[method.0 as usize].clone()
            }
            Member::Attribute => {
                let info = &self.classes[protocol.0 as usize];
                let returns = info
                    .attribute(name)
                    .and_then(|index| info.attributes[index].ty.clone())
                    .unwrap_or(Type::Error);
                FunctionSignature {
                    parameters: vec![SignatureParameter {
                        name: "self".to_owned(),
                        ty: instance,
                        default: None,
                        default_call: None,
                        kind: typy_source::syntax::ParameterKind::Normal,
                    }],
                    returns,
                }
            }
        };
        let id = self.next_function();
        self.functions.push(None);
        self.signatures.push(signature);
        self.protocol_values
            .dispatchers
            .insert((protocol, name.to_owned()), id);
    }

    /// Writes the body of every dispatcher: a switch over the classes that satisfy its
    /// protocol, the most derived first, each arm reaching the member of the object's class.
    pub(crate) fn write_protocol_dispatchers(&mut self) {
        let dispatchers: Vec<((ClassId, String), FunctionId)> = self
            .protocol_values
            .dispatchers
            .iter()
            .map(|(key, id)| (key.clone(), *id))
            .collect();
        for ((protocol, name), id) in dispatchers {
            let member = if self.classes[protocol.0 as usize].method(&name).is_some() {
                Member::Method
            } else {
                Member::Attribute
            };
            let function = self.dispatcher_body(protocol, &name, member, id);
            self.functions[id.0 as usize] = Some(function);
        }
    }

    /// Returns the dispatcher `id` of the member `name` of `protocol`, with its body.
    fn dispatcher_body(
        &mut self,
        protocol: ClassId,
        name: &str,
        member: Member,
        id: FunctionId,
    ) -> TypedFunction {
        let signature = self.signatures[id.0 as usize].clone();
        let info = &self.classes[protocol.0 as usize];
        let (protocol_name, span) = (info.name.clone(), info.span);
        let module = self.class_modules[protocol.0 as usize].clone();
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
        let typed = |kind: TypedExprKind, ty: Type| TypedExpr { kind, ty, span };
        let receiver = typed(
            TypedExprKind::Local(symbols[0]),
            signature.parameters[0].ty.clone(),
        );
        let mut arms = Vec::new();
        for class in self.satisfying_classes(protocol) {
            let Some(arm) = self.dispatcher_arm(class, name, member, (&signature, &symbols), span)
            else {
                continue;
            };
            let of = TypeObjectOf::Instance(class, self.classes[class.0 as usize].name.clone());
            arms.push((MemberTest::Class(of), arm));
        }
        // No value of the protocol is made when no class satisfies it, so the body never runs.
        let statement = if arms.is_empty() {
            TypedStmt::Raise {
                exception: None,
                cause: crate::typed::RaiseCause::Absent,
                span,
            }
        } else {
            TypedStmt::Return {
                value: Some(typed(
                    TypedExprKind::EachMember(Box::new(EachMember {
                        value: receiver,
                        arms,
                    })),
                    signature.returns.clone(),
                )),
                span,
            }
        };
        TypedFunction {
            id,
            name: name.to_owned(),
            qualified_name: format!("<protocol {protocol_name}.{name}>"),
            module,
            signature,
            parameters: symbols,
            locals: Vec::new(),
            body: vec![statement],
            span,
            generic: false,
            named_targets: Vec::new(),
            generator: None,
        }
    }

    /// Returns the classes that satisfy `protocol`, each class derived from another before it.
    pub(super) fn satisfying_classes(&self, protocol: ClassId) -> Vec<ClassId> {
        let mut classes: Vec<ClassId> = self
            .protocol_values
            .fits
            .iter()
            .filter(|(_, each)| *each == protocol)
            .map(|(class, _)| *class)
            .collect();
        let depth = |class: ClassId| {
            std::iter::successors(Some(class), |each| self.classes[each.0 as usize].base).count()
        };
        classes.sort_by_key(|class| std::cmp::Reverse(depth(*class)));
        classes
    }

    /// Returns the arm of a dispatcher for `class`: its method called with the dispatcher's
    /// arguments, each converted to the method's type, and its result converted to the
    /// dispatcher's; or its attribute read.
    fn dispatcher_arm(
        &self,
        class: ClassId,
        name: &str,
        member: Member,
        (signature, symbols): (&FunctionSignature, &[SymbolId]),
        span: Span,
    ) -> Option<TypedExpr> {
        let typed = |kind: TypedExprKind, ty: Type| TypedExpr { kind, ty, span };
        let instance = self.instance_type(class);
        let object = typed(
            TypedExprKind::Downcast(Box::new(typed(
                TypedExprKind::Local(symbols[0]),
                signature.parameters[0].ty.clone(),
            ))),
            instance.clone(),
        );
        let reached = match member {
            Member::Method => {
                let method = self.method(class, name)?;
                let own = &self.signatures[method.0 as usize];
                let mut arguments = vec![Argument {
                    passed: crate::typed::Passed::Position,
                    parameter: 0,
                    value: BodyChecker::converted(
                        object,
                        own.parameters.first().map(|each| &each.ty),
                    ),
                }];
                for (position, (parameter, symbol)) in
                    signature.parameters.iter().zip(symbols).enumerate().skip(1)
                {
                    let value = typed(TypedExprKind::Local(*symbol), parameter.ty.clone());
                    let wanted = own.parameters.get(position).map(|each| &each.ty);
                    arguments.push(Argument {
                        passed: crate::typed::Passed::Position,
                        parameter: position,
                        value: BodyChecker::converted(value, wanted),
                    });
                }
                typed(
                    TypedExprKind::Call {
                        function: method,
                        arguments,
                        dispatch: true,
                    },
                    own.returns.clone(),
                )
            }
            Member::Attribute => {
                let index = self.classes[class.0 as usize].attribute(name)?;
                let ty = self.attribute_ty(class, index)?;
                typed(
                    TypedExprKind::Attribute {
                        object: Box::new(object),
                        index,
                    },
                    ty,
                )
            }
        };
        Some(BodyChecker::converted(reached, Some(&signature.returns)))
    }
}

impl BodyChecker<'_> {
    /// Reads `object.name` on a protocol value: a call of the attribute's getter dispatcher.
    pub(super) fn protocol_attribute(
        &mut self,
        object: TypedExpr,
        protocol: ClassId,
        attribute: &Identifier,
        span: Span,
    ) -> TypedExpr {
        let info = &self.program.classes[protocol.0 as usize];
        let getter = self
            .program
            .dispatcher(protocol, &attribute.name)
            .filter(|_| info.attribute(&attribute.name).is_some());
        let Some(getter) = getter else {
            if let Some(method) = info.method(&attribute.name) {
                return self.protocol_method_value(object, (protocol, method), attribute, span);
            }
            let message = format!("`{}` has no attribute `{}`", info.name, attribute.name);
            return self.error(
                Diagnostic::new(DiagnosticCode::error(414), attribute.span, message)
                    .with_rule(RuleId::GEN_10),
                span,
            );
        };
        let returns = self.program.signatures[getter.0 as usize].returns.clone();
        Self::typed(
            TypedExprKind::Call {
                function: getter,
                arguments: vec![Argument {
                    passed: crate::typed::Passed::Position,
                    parameter: 0,
                    value: object,
                }],
                dispatch: false,
            },
            returns,
            span,
        )
    }
}

impl BodyChecker<'_> {
    /// Reports an assignment of an attribute through a protocol value, planned for 1.0
    /// (GEN-17); returns true when it reports one.
    pub(super) fn refuse_through_protocol(
        &mut self,
        class: ClassId,
        attribute: &Identifier,
    ) -> bool {
        if !self.program.is_protocol(class) {
            return false;
        }
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(201),
                attribute.span,
                "assigning an attribute through a protocol value is planned for TyPy 0.13 (GEN-17)",
            )
            .with_rule(RuleId::GEN_17),
        );
        true
    }

    /// Returns why a value of type `actual` cannot be held where `expected`, a protocol or a
    /// type that holds one, is wanted, for the help of the report that refuses it.
    pub(super) fn protocol_help(&self, actual: &Type, expected: &Type) -> Option<String> {
        let protocol = match expected {
            Type::Instance(class) if class.protocol => class.id,
            Type::Optional(inner) => return self.protocol_help(actual, inner),
            Type::Union(members) => {
                return members
                    .iter()
                    .find_map(|member| self.protocol_help(actual, member));
            }
            _ => return None,
        };
        let name = &self.program.classes[protocol.0 as usize].name;
        match actual {
            Type::Instance(class) if !class.protocol => self
                .program
                .class_unsatisfied(class.id, protocol)
                .map(|(reason, _)| format!("it does not satisfy the protocol `{name}`: {reason}")),
            Type::Error | Type::Instance(_) | Type::Union(_) | Type::Optional(_) => None,
            other => Some(format!(
                "{} held as the protocol `{name}` is planned for TyPy 0.13 (GEN-17)",
                other.with_article()
            )),
        }
    }
}
