//! Function and method definitions and their signatures (TYP-01, CLS-01).

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Identifier, Parameter, Stmt};

use super::annotation::{AnnotationScope, annotation_type};
use crate::typed::ClassId;
use crate::typed::{FunctionId, FunctionSignature, SignatureParameter, TypedConstant};
use crate::types::Type;

/// A function definition with its signature: a module-level function, or a method of a class.
#[derive(Clone, Debug)]
pub(crate) struct Definition<'tree> {
    pub(crate) name: String,
    /// The name, or for a method the class's name, a dot and the name.
    pub(crate) qualified_name: String,
    /// The class of a method.
    pub(crate) class: Option<ClassId>,
    /// Whether the first parameter is the instance: a method other than a static method or a
    /// class method.
    pub(crate) takes_instance: bool,
    /// For a copy of a class method a class inherits, the copy of the class that defines it,
    /// whose diagnostics this copy does not repeat (ADR-0025).
    pub(crate) copy_of: Option<FunctionId>,
    pub(crate) signature: FunctionSignature,
    /// The type parameters of a generic function, whose copies run in its place (GEN-01).
    pub(crate) type_parameters: Vec<crate::generic::TypeParameter>,
    pub(crate) parameters: Vec<(String, Span)>,
    pub(crate) body: &'tree [Stmt],
    pub(crate) span: Span,
}

/// The parts of a `def` statement a definition is built from.
pub(super) struct FunctionParts<'tree> {
    pub(super) name: &'tree Identifier,
    /// The decorators, of which a method may have one of CLS-06.
    pub(super) decorators: &'tree [Expr],
    pub(super) parameters: &'tree [Parameter],
    /// The type parameters of a generic function (GEN-01).
    pub(super) type_parameters: &'tree [typy_source::syntax::TypeParameter],
    pub(super) returns: Option<&'tree Expr>,
    pub(super) body: &'tree [Stmt],
    pub(super) span: Span,
}

/// What a decorator makes of a method (CLS-06).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum MethodKind {
    /// A method of the instance, without a decorator.
    Instance,
    /// `@staticmethod`: a function of the class, without the instance.
    Static,
    /// `@classmethod`: a function of the class, which takes it.
    Class,
    /// `@property`: read as the attribute of its name.
    Getter,
    /// `@name.setter`: called when the attribute `name` is assigned.
    Setter(String),
}

impl<'tree> FunctionParts<'tree> {
    /// Returns what the method's decorator makes of it.
    pub(super) fn method_kind(&self) -> MethodKind {
        match self.decorators.first() {
            Some(Expr::Name { id, .. }) if id == "staticmethod" => MethodKind::Static,
            Some(Expr::Name { id, .. }) if id == "classmethod" => MethodKind::Class,
            Some(Expr::Name { id, .. }) if id == "property" => MethodKind::Getter,
            Some(Expr::Attribute { value, .. }) => match value.as_ref() {
                Expr::Name { id, .. } => MethodKind::Setter(id.clone()),
                _ => MethodKind::Instance,
            },
            _ => MethodKind::Instance,
        }
    }

    /// Returns the parts of a `def` statement, or `None` for any other statement.
    pub(super) fn of(statement: &'tree Stmt) -> Option<Self> {
        let Stmt::FunctionDef {
            name,
            parameters,
            returns,
            body,
            decorators,
            type_parameters,
            span,
            ..
        } = statement
        else {
            return None;
        };
        Some(Self {
            name,
            decorators,
            parameters,
            type_parameters,
            returns: returns.as_ref(),
            body,
            span: *span,
        })
    }
}

/// Reports a second definition of a name; TyPy keeps one definition per name.
pub(super) fn defined_twice(name: &Identifier) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(202),
        name.span,
        format!(
            "`{}` is defined more than once; TyPy keeps one definition per name",
            name.name
        ),
    )
    .with_rule(RuleId::STD_04)
}

/// What a module's function signatures may name: its classes, with the type of their
/// instances, the names its imports give, and the generic classes and their copies.
#[derive(Clone, Copy)]
pub(super) struct FunctionScope<'known> {
    pub(super) classes: &'known BTreeMap<String, ClassId>,
    pub(super) instances: &'known BTreeMap<ClassId, Type>,
    pub(super) names: &'known crate::check::ImportedNames,
    pub(super) copies: &'known std::cell::RefCell<crate::body::ClassCopies>,
}

/// Collects the module's functions in source order, with their signatures (TYP-01). Each
/// signature may name the classes whose `class` statements come before its `def`, and the
/// classes of the modules checked before it, numbered below `first_class`.
pub(super) fn collect_functions<'tree>(
    statements: &'tree [Stmt],
    first_class: usize,
    known: &FunctionScope<'_>,
    floor_defers: bool,
    diagnostics: &mut Diagnostics,
) -> Vec<Definition<'tree>> {
    let FunctionScope {
        classes,
        instances,
        names,
        copies,
    } = *known;
    let mut functions: Vec<Definition<'tree>> = Vec::new();
    let mut classes_before = u32::try_from(first_class).unwrap_or(u32::MAX);
    for statement in statements {
        if matches!(statement, Stmt::ClassDef { .. }) {
            classes_before += 1;
        }
        let Some(parts) = FunctionParts::of(statement) else {
            continue;
        };
        if functions
            .iter()
            .any(|existing| existing.name == parts.name.name)
        {
            diagnostics.push(defined_twice(parts.name));
            continue;
        }
        let mut scope = AnnotationScope {
            classes,
            instances,
            modules: &names.modules,
            module_aliases: &names.module_aliases,
            imported_aliases: &names.aliases,
            names: &names.names,
            standard: &names.standard,
            parameters: &super::annotation::NO_PARAMETERS,
            copies,
            bound: if floor_defers {
                u32::MAX
            } else {
                classes_before
            },
        };
        let type_parameters = type_parameters(&parts, &scope, diagnostics);
        let in_scope: BTreeMap<String, Type> = type_parameters
            .iter()
            .map(|parameter| (parameter.name.clone(), Type::parameter(parameter.clone())))
            .collect();
        scope.parameters = &in_scope;
        let mut made = definition(&parts, None, &scope, diagnostics);
        made.type_parameters = type_parameters;
        super::protocol::protocol_parameters(&mut made, diagnostics);
        functions.push(made);
    }
    functions
}

/// Returns the type parameters of a generic function, each with its bound or its
/// constraints (GEN-01, GEN-02).
fn type_parameters(
    parts: &FunctionParts<'_>,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Vec<crate::generic::TypeParameter> {
    resolve_type_parameters(parts.type_parameters, parts.span, scope, diagnostics)
}

/// Returns the type parameters a list declares for the definition at `owner`, each with its
/// bound or its constraints (GEN-01, GEN-02).
pub(super) fn resolve_type_parameters(
    parameters: &[typy_source::syntax::TypeParameter],
    owner: Span,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Vec<crate::generic::TypeParameter> {
    parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            let (bound, constraints) = match &parameter.bound {
                Some(Expr::Tuple { elements, .. }) => (
                    None,
                    elements
                        .iter()
                        .map(|each| annotation_type(each, scope, diagnostics))
                        .collect(),
                ),
                Some(bound) => (Some(annotation_type(bound, scope, diagnostics)), Vec::new()),
                None => (None, Vec::new()),
            };
            crate::generic::TypeParameter {
                owner,
                index,
                name: parameter.name.name.clone(),
                bound,
                constraints,
            }
        })
        .collect()
}

/// The class a method belongs to, which types its first parameter.
pub(super) struct Owner {
    pub(super) id: ClassId,
    /// The type of the class's instances.
    pub(super) instance: Type,
    /// Whether the first parameter is the instance: false for a static method (CLS-06).
    pub(super) takes_instance: bool,
}

/// Returns the type of a method's first parameter, the instance, after reporting an
/// annotation that names another type.
fn instance_parameter(
    owner: &Owner,
    annotation: Option<&Expr>,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    let instance = owner.instance.clone();
    // A class method's first parameter is the class each copy is made for (ADR-0025).
    if let Some(annotation) = annotation.filter(|_| !matches!(instance, Type::TypeObject(_))) {
        let annotated = annotation_type(annotation, scope, diagnostics);
        if !annotated.matches(&instance) {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    annotation.span(),
                    format!(
                        "the first parameter of a method is the instance, {}",
                        instance.with_article()
                    ),
                )
                .with_rule(RuleId::CLS_01),
            );
        }
    }
    instance
}

/// Returns the definition of a function, or of a method of `owner`, whose first parameter is
/// the instance and needs no annotation.
pub(super) fn definition<'tree>(
    parts: &FunctionParts<'tree>,
    owner: Option<&Owner>,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Definition<'tree> {
    let name = &parts.name.name;
    let mut signature_parameters = Vec::new();
    let mut parameter_names = Vec::new();
    for (index, parameter) in parts.parameters.iter().enumerate() {
        let ty = match (owner, &parameter.annotation) {
            (Some(owner), annotation) if index == 0 && owner.takes_instance => {
                instance_parameter(owner, annotation.as_ref(), scope, diagnostics)
            }
            (_, Some(annotation)) => annotation_type(annotation, scope, diagnostics),
            // The parameters of `__exit__` have the types CPython gives them (CST-10).
            (Some(owner), None)
                if owner.takes_instance && name == "__exit__" && (1..=3).contains(&index) =>
            {
                exit_parameter(index)
            }
            (_, None) => {
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(203),
                        parameter.span,
                        format!("parameter `{}` has no type annotation", parameter.name.name),
                    )
                    .with_rule(RuleId::TYP_01)
                    .with_help(format!(
                        "write `{}: int`, or the type it holds",
                        parameter.name.name
                    )),
                );
                Type::Error
            }
        };
        let default_call = parameter.default.as_ref().and_then(factory_call);
        let default = parameter
            .default
            .as_ref()
            .filter(|_| default_call.is_none())
            .and_then(|default| default_value(default, &ty, diagnostics));
        signature_parameters.push(SignatureParameter {
            name: parameter.name.name.clone(),
            ty,
            default,
            default_call,
        });
        parameter_names.push((parameter.name.name.clone(), parameter.name.span));
    }
    let initializer = owner.is_some() && name == "__init__";
    let returns = match parts.returns {
        Some(annotation) => annotation_type(annotation, scope, diagnostics),
        // `__init__` returns `None`, so it may leave its return annotation out, as mypy allows.
        None if initializer => Type::NoneType,
        None => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(204),
                    parts.name.span,
                    format!("function `{name}` has no return annotation"),
                )
                .with_rule(RuleId::TYP_01)
                .with_help("write `-> None` if it returns nothing"),
            );
            Type::Error
        }
    };
    if initializer && !returns.matches(&Type::NoneType) {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(405),
                parts.returns.map_or(parts.name.span, Expr::span),
                "`__init__` returns `None`",
            )
            .with_rule(RuleId::CLS_01),
        );
    }
    Definition {
        name: name.clone(),
        qualified_name: owner.map_or_else(
            || name.clone(),
            |owner| format!("{}.{name}", owner.instance),
        ),
        class: owner.map(|owner| owner.id),
        takes_instance: owner.is_some_and(|owner| owner.takes_instance)
            && !matches!(
                owner.map(|owner| &owner.instance),
                Some(Type::TypeObject(_))
            ),
        copy_of: None,
        type_parameters: Vec::new(),
        signature: FunctionSignature {
            parameters: signature_parameters,
            returns,
        },
        parameters: parameter_names,
        body: parts.body,
        span: parts.span,
    }
}

/// Returns the name of the function a dataclass's `__init__` calls for a field with a
/// `default_factory`, when `default` is its call (CLS-15). Its name holds `<`, so no
/// program can write it.
pub(crate) fn factory_call(default: &Expr) -> Option<String> {
    let Expr::Call {
        function,
        arguments,
        keywords,
        ..
    } = default
    else {
        return None;
    };
    match function.as_ref() {
        Expr::Name { id, .. }
            if id.starts_with("<default ") && arguments.is_empty() && keywords.is_empty() =>
        {
            Some(id.clone())
        }
        _ => None,
    }
}

/// Returns the constant of a default value, checked against the parameter's type (TYP-09).
fn default_value(
    default: &Expr,
    ty: &Type,
    diagnostics: &mut Diagnostics,
) -> Option<TypedConstant> {
    let Expr::Constant { value, span } = default else {
        return None;
    };
    let constant = match value {
        Constant::None => TypedConstant::None,
        Constant::Bool(value) => TypedConstant::Bool(*value),
        Constant::Int(digits) => TypedConstant::Int(digits.clone()),
        Constant::Float(value) => TypedConstant::Float(*value),
        Constant::Str(value) => TypedConstant::Str(value.clone()),
        Constant::StrPoints(points) => TypedConstant::StrPoints(points.clone()),
        Constant::Complex { .. } | Constant::Bytes(_) | Constant::Ellipsis => return None,
    };
    if !constant.ty().is_assignable_to(ty) {
        let mut diagnostic = Diagnostic::new(
            DiagnosticCode::error(404),
            *span,
            format!(
                "the default value is {}, but the parameter is {}",
                constant.ty().with_article(),
                ty.with_article()
            ),
        )
        .with_rule(if *ty == Type::Float {
            RuleId::TYP_09
        } else {
            RuleId::TYP_03
        });
        if let (TypedConstant::Int(digits), Type::Float) = (&constant, ty) {
            diagnostic = diagnostic.with_fix(*span, format!("{digits}.0"));
        }
        diagnostics.push(diagnostic);
    }
    Some(constant)
}

/// Returns the type of parameter `index` of `__exit__` after `self`: the class of the
/// exception, the exception and its traceback, each `None` when the block raised nothing
/// (CST-10).
pub(crate) fn exit_parameter(index: usize) -> Type {
    use crate::exception::{BuiltinException, ExceptionClass};
    Type::optional(match index {
        // The class of an exception is known only when the program runs, so
        // `type[BaseException]` holds any type object, as it does in an annotation.
        1 => Type::TypeObject(None),
        2 => Type::Exception(ExceptionClass::Builtin(BuiltinException::BaseException)),
        _ => Type::Traceback,
    })
}
