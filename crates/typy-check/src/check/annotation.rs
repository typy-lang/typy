//! The types that annotations name: `int`, `float`, `bool`, `str`, `None`, and from 0.2
//! `tuple[...]`.

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId};
use typy_source::syntax::{Constant, Expr};

use crate::abstract_type::AbstractCollection;
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::ClassId;
use crate::types::Type;

/// The classes an annotation may name. A class whose ID is below `bound` is bound where the
/// annotation is evaluated, so its bare name may be used; any class may be named in a string.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AnnotationScope<'classes> {
    pub(crate) classes: &'classes BTreeMap<String, ClassId>,
    /// The type of each class's instances: an exception for an exception class.
    pub(crate) instances: &'classes BTreeMap<ClassId, Type>,
    /// For each name bound to a module at module level, that module's classes, which an
    /// annotation such as `shapes.Point` names (MOD-10).
    pub(crate) modules: &'classes BTreeMap<String, BTreeMap<String, ClassId>>,
    /// For each of those names, that module's type aliases (SMT-10, SMT-12).
    pub(crate) module_aliases: &'classes BTreeMap<String, AliasTable>,
    /// The type aliases of other modules imported by name (SMT-12).
    pub(crate) imported_aliases: &'classes AliasTable,
    /// The names imported from `typing`, `collections.abc` and `types` (TYP-15).
    pub(crate) names: &'classes BTreeMap<String, AnnotationName>,
    /// For each name bound to one of those modules, the module.
    pub(crate) standard: &'classes BTreeMap<String, String>,
    /// The type each type parameter in scope stands for: its opaque type while a generic
    /// body is checked, and its argument while a copy is (GEN-01).
    pub(crate) parameters: &'classes BTreeMap<String, Type>,
    /// The generic classes and their copies, which an annotation such as `Box[int]` asks
    /// for (ADR-0028).
    pub(crate) copies: &'classes std::cell::RefCell<crate::body::ClassCopies>,
    pub(crate) bound: u32,
}

/// A type alias of a module as another module reads it: its type parameters, its value with
/// its module's own aliases expanded, and what its module's annotations see, so the value is
/// read where it was written (SMT-12).
#[derive(Debug, PartialEq)]
pub(crate) struct ExportedAlias {
    /// The module that defines the alias, and the alias's own name there.
    pub(crate) module: String,
    pub(crate) name: String,
    pub(crate) parameters: Vec<String>,
    pub(crate) value: Expr,
    pub(crate) classes: BTreeMap<String, ClassId>,
    pub(crate) names: crate::check::ImportedNames,
}

/// Type aliases by the name a module reads them under.
pub(crate) type AliasTable = BTreeMap<String, std::rc::Rc<ExportedAlias>>;

/// The type parameters of a scope that has none.
pub(crate) static NO_PARAMETERS: BTreeMap<String, Type> = BTreeMap::new();

/// A name of `typing`, `collections.abc` or `types` that TyPy 0.3 gives for annotations
/// (TYP-15, CST-10, ADR-0015).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnnotationName {
    Iterator(Origin),
    Iterable(Origin),
    /// `Generator[Y, S, R]`, the type of a generator object (FUN-07).
    Generator(Origin),
    TracebackType,
    ModuleType,
    /// `typing.ClassVar`, which declares a class variable in a class body (CLS-11).
    ClassVar,
    /// `typing.Final`, which declares a name that is not assigned again (TYP-08).
    Final,
    /// `dataclasses.dataclass` and `dataclasses.field`, which the dataclass expansion reads
    /// (CLS-05).
    Dataclasses,
    /// `typing.Protocol`, the base of a protocol class (GEN-04).
    Protocol,
    /// `typing.cast`, a function a call checks when the program runs (TYP-24).
    Cast,
    /// `typing.NamedTuple`, the base of a named tuple, which the dataclass expansion reads
    /// (CLS-27).
    NamedTuple,
    /// A name that imports bind to more than one class, whose type depends on the import
    /// that ran last (MOD-14).
    Rebound,
    /// `typing.Optional`, `Optional[T]` being `T | None` (LIB-17).
    Optional,
    /// `typing.Union`, `Union[A, B]` being `A | B` (LIB-17).
    Union,
    /// `typing.List`, `Dict`, `Set`, `Tuple` and `Type`, the built-in type of the same name
    /// in lower case (LIB-17).
    Builtin(&'static str),
    /// `Callable` of `typing` or `collections.abc`, the type of a function value (FUN-05).
    Callable(Origin),
    /// An abstract collection of `typing` or `collections.abc`, such as `Sequence`, held as
    /// the union of the built-in types CPython registers with it (LIB-20, ADR-0086).
    Abstract(AbstractCollection, Origin),
}

/// The module a name of both `typing` and `collections.abc` comes from: the class of
/// `collections.abc`, or the alias of `typing`, which no class pattern takes (TYP-17, TYP-21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Origin {
    Typing,
    Abc,
}

/// The modules that give names for annotations only in 0.3.
pub(crate) const ANNOTATION_MODULES: [&str; 4] =
    ["typing", "collections.abc", "types", "dataclasses"];

/// Returns the annotation name `name` of the module `module`, if TyPy 0.3 gives it.
pub(crate) fn annotation_name(module: &str, name: &str) -> Option<AnnotationName> {
    match (module, name) {
        ("typing" | "collections.abc", "Iterator") => {
            Some(AnnotationName::Iterator(origin(module)))
        }
        ("typing" | "collections.abc", "Iterable") => {
            Some(AnnotationName::Iterable(origin(module)))
        }
        ("typing" | "collections.abc", "Generator") => {
            Some(AnnotationName::Generator(origin(module)))
        }
        ("typing" | "collections.abc", "Callable") => {
            Some(AnnotationName::Callable(origin(module)))
        }
        ("types", "TracebackType") => Some(AnnotationName::TracebackType),
        ("types", "ModuleType") => Some(AnnotationName::ModuleType),
        ("typing", "ClassVar") => Some(AnnotationName::ClassVar),
        ("typing", "Final") => Some(AnnotationName::Final),
        ("typing", "Protocol") => Some(AnnotationName::Protocol),
        ("typing", "cast") => Some(AnnotationName::Cast),
        ("typing", "NamedTuple") => Some(AnnotationName::NamedTuple),
        ("typing", "Optional") => Some(AnnotationName::Optional),
        ("typing", "Union") => Some(AnnotationName::Union),
        ("typing", "List") => Some(AnnotationName::Builtin("list")),
        ("typing", "Dict") => Some(AnnotationName::Builtin("dict")),
        ("typing", "Set") => Some(AnnotationName::Builtin("set")),
        ("typing", "Tuple") => Some(AnnotationName::Builtin("tuple")),
        ("typing", "Type") => Some(AnnotationName::Builtin("type")),
        ("dataclasses", "dataclass" | "field") => Some(AnnotationName::Dataclasses),
        ("typing" | "collections.abc", _) => AbstractCollection::named(module, name)
            .map(|collection| AnnotationName::Abstract(collection, origin(module))),
        _ => None,
    }
}

/// Returns where a name of both `typing` and `collections.abc` comes from.
fn origin(module: &str) -> Origin {
    if module == "typing" {
        Origin::Typing
    } else {
        Origin::Abc
    }
}

/// Returns true when an expression names `typing.Protocol`, as the base of a protocol class
/// does (GEN-04).
pub(crate) fn is_protocol(
    expression: &Expr,
    names: &BTreeMap<String, AnnotationName>,
    standard: &BTreeMap<String, String>,
) -> bool {
    match expression {
        Expr::Name { id, .. } => names.get(id) == Some(&AnnotationName::Protocol),
        Expr::Attribute {
            value, attribute, ..
        } => {
            attribute.name == "Protocol"
                && matches!(value.as_ref(), Expr::Name { id, .. }
                    if standard.get(id).is_some_and(|module| module == "typing"))
        }
        _ => false,
    }
}

/// Returns the annotation name an expression names: an imported name, or a name read
/// through one of the modules, such as `typing.Iterator` or `collections.abc.Iterable`.
pub(crate) fn standard_name(
    expression: &Expr,
    scope: &AnnotationScope<'_>,
) -> Option<AnnotationName> {
    match expression {
        Expr::Name { id, .. } => scope.names.get(id).copied(),
        Expr::Attribute {
            value, attribute, ..
        } => {
            let module = standard_module(value, scope)?;
            annotation_name(&module, &attribute.name)
        }
        _ => None,
    }
}

/// Returns the type a name of `typing`, `collections.abc` or `types` names, with its items
/// for `Iterator[T]` and `Iterable[T]`, or `None` for any other annotation (TYP-15).
fn standard_annotation(
    annotation: &Expr,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Option<Type> {
    Some(match annotation {
        Expr::Subscript { value, index, .. }
            if let Some(name @ (AnnotationName::Iterator(_) | AnnotationName::Iterable(_))) =
                standard_name(value, scope) =>
        {
            let item = Box::new(annotation_type(index, scope, diagnostics));
            if let AnnotationName::Iterator(_) = name {
                Type::Iterator(item)
            } else {
                Type::Iterable(item)
            }
        }
        Expr::Subscript { value, index, span }
            if matches!(
                standard_name(value, scope),
                Some(AnnotationName::Callable(_))
            ) =>
        {
            super::callable_annotation::callable_annotation(index, *span, scope, diagnostics)
        }
        Expr::Subscript { value, index, .. }
            if matches!(
                standard_name(value, scope),
                Some(AnnotationName::Generator(_))
            ) =>
        {
            generator_annotation(index, scope, diagnostics)
        }
        // `Sequence[int]` is the union of the built-in sequences of `int` (LIB-20).
        Expr::Subscript { value, index, span }
            if let Some(AnnotationName::Abstract(collection, _)) = standard_name(value, scope) =>
        {
            let written: Vec<&Expr> = match index.as_ref() {
                Expr::Tuple { elements, .. } => elements.iter().collect(),
                other => vec![other],
            };
            let arguments: Vec<Type> = written
                .iter()
                .map(|each| annotation_type(each, scope, diagnostics))
                .collect();
            if arguments.iter().any(Type::has_error) {
                return Some(Type::Error);
            }
            let Some(ty) = collection.of(&arguments) else {
                diagnostics.push(abstract_form(collection, *span));
                return Some(Type::Error);
            };
            ty
        }
        // `List[int]` is `list[int]`, read as the built-in type is (LIB-17).
        Expr::Subscript { value, index, span }
            if let Some(AnnotationName::Builtin(builtin)) = standard_name(value, scope) =>
        {
            let spelled = Expr::Subscript {
                value: Box::new(Expr::Name {
                    id: builtin.to_owned(),
                    span: value.span(),
                }),
                index: index.clone(),
                span: *span,
            };
            annotation_type(&spelled, scope, diagnostics)
        }
        Expr::Subscript { value, index, .. }
            if let Some(name @ (AnnotationName::Optional | AnnotationName::Union)) =
                standard_name(value, scope) =>
        {
            union_annotation(name, index, scope, diagnostics)
        }
        Expr::Subscript { value, .. }
            if standard_name(value, scope) == Some(AnnotationName::ClassVar) =>
        {
            class_var_elsewhere(annotation, diagnostics)
        }
        Expr::Subscript { value, .. }
            if standard_name(value, scope) == Some(AnnotationName::Final) =>
        {
            final_elsewhere(annotation, diagnostics)
        }
        _ if let Some(name) = standard_name(annotation, scope) => {
            bare_standard(name, annotation, scope, diagnostics)
        }
        _ => return None,
    })
}

/// Returns the type a bare name of `typing`, `collections.abc` or `types` names, or the error
/// for one that names no type without its arguments.
fn bare_standard(
    name: AnnotationName,
    annotation: &Expr,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    match name {
        AnnotationName::ClassVar => class_var_elsewhere(annotation, diagnostics),
        AnnotationName::Final => final_elsewhere(annotation, diagnostics),
        AnnotationName::Dataclasses | AnnotationName::Cast => {
            not_a_type(&name, annotation.span(), diagnostics)
        }
        AnnotationName::Protocol => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(408),
                    annotation.span(),
                    "`Protocol` is not a type; a protocol class derives from it",
                )
                .with_rule(RuleId::GEN_04),
            );
            Type::Error
        }
        AnnotationName::NamedTuple => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(408),
                    annotation.span(),
                    "`NamedTuple` is not a type in TyPy; a named tuple's class derives from it",
                )
                .with_rule(RuleId::CLS_27)
                .with_help("annotate the value with the named tuple's class"),
            );
            Type::Error
        }
        AnnotationName::Rebound => {
            let Expr::Name { id, .. } = annotation else {
                return Type::Error;
            };
            diagnostics.push(crate::body::rebound_planned(
                id,
                "named in an annotation",
                annotation.span(),
            ));
            Type::Error
        }
        AnnotationName::Builtin(builtin) => {
            let spelled = Expr::Name {
                id: builtin.to_owned(),
                span: annotation.span(),
            };
            annotation_type(&spelled, scope, diagnostics)
        }
        AnnotationName::Optional | AnnotationName::Union => {
            let written = if name == AnnotationName::Optional {
                "an `Optional` annotation names its type, such as `Optional[int]`"
            } else {
                "a `Union` annotation names its types, such as `Union[int, str]`"
            };
            diagnostics.push(
                Diagnostic::new(DiagnosticCode::error(408), annotation.span(), written)
                    .with_rule(RuleId::LIB_17),
            );
            Type::Error
        }
        AnnotationName::Callable(_) => {
            diagnostics.push(super::callable_annotation::callable_form(annotation.span()));
            Type::Error
        }
        AnnotationName::Abstract(collection, _) => {
            diagnostics.push(abstract_form(collection, annotation.span()));
            Type::Error
        }
        AnnotationName::TracebackType => Type::Traceback,
        AnnotationName::ModuleType => Type::AnyModule,
        AnnotationName::Iterator(_) | AnnotationName::Iterable(_) => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(408),
                    annotation.span(),
                    "an `Iterator` or `Iterable` annotation names the type of its items, such as `Iterator[int]`",
                )
                .with_rule(RuleId::TYP_15),
            );
            Type::Error
        }
        AnnotationName::Generator(_) => {
            diagnostics.push(generator_form(annotation.span()));
            Type::Error
        }
    }
}

/// Returns E0408 for an abstract collection without the types of its items, or with another
/// number of them than it takes.
fn abstract_form(collection: AbstractCollection, span: typy_diagnostics::Span) -> Diagnostic {
    let example = match collection {
        AbstractCollection::Mapping | AbstractCollection::MutableMapping => {
            format!("{}[str, int]", collection.name())
        }
        other => format!("{}[int]", other.name()),
    };
    Diagnostic::new(
        DiagnosticCode::error(408),
        span,
        format!(
            "a `{}` annotation names the types of its items, such as `{example}`",
            collection.name()
        ),
    )
    .with_rule(RuleId::LIB_20)
}

/// Returns the type `Generator[Y, S, R]` names: what it yields, what it is sent and what it
/// returns, each given, as CPython 3.12 needs them (FUN-07).
fn generator_annotation(
    index: &Expr,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    let Expr::Tuple { elements, span } = index else {
        diagnostics.push(generator_form(index.span()));
        return Type::Error;
    };
    let [yields, sends, returns] = elements.as_slice() else {
        diagnostics.push(generator_form(*span));
        return Type::Error;
    };
    Type::Generator {
        yields: Box::new(annotation_type(yields, scope, diagnostics)),
        sends: Box::new(annotation_type(sends, scope, diagnostics)),
        returns: Box::new(annotation_type(returns, scope, diagnostics)),
    }
}

/// Returns E0408 for a `Generator` annotation without its three types.
fn generator_form(span: typy_diagnostics::Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(408),
        span,
        "a `Generator` annotation names what it yields, what it is sent and what it returns, \
         such as `Generator[int, None, None]`",
    )
    .with_rule(RuleId::FUN_07)
}

/// Returns the type `Optional[T]`, which is `T | None`, or `Union[A, B, ...]`, which is
/// `A | B | ...`, names (LIB-17).
fn union_annotation(
    name: AnnotationName,
    index: &Expr,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    let members: Vec<&Expr> = match index {
        Expr::Tuple { elements, .. } => elements.iter().collect(),
        other => vec![other],
    };
    if name == AnnotationName::Optional && members.len() != 1 {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(408),
                index.span(),
                "`Optional` takes one type, as CPython requires",
            )
            .with_rule(RuleId::LIB_17),
        );
        return Type::Error;
    }
    let mut types: Vec<Type> = members
        .into_iter()
        .map(|member| annotation_type(member, scope, diagnostics))
        .collect();
    if name == AnnotationName::Optional {
        types.push(Type::NoneType);
    }
    Type::union(types)
}

/// Reports `Final` outside the declaration of a variable, a class variable or an attribute
/// given a value.
fn final_elsewhere(annotation: &Expr, diagnostics: &mut Diagnostics) -> Type {
    diagnostics.push(
        Diagnostic::new(
            DiagnosticCode::error(408),
            annotation.span(),
            "`Final` declares a variable, a class variable or an attribute given a value",
        )
        .with_rule(RuleId::TYP_08),
    );
    Type::Error
}

/// Reports `ClassVar` outside the declaration of a class variable in a class body.
fn class_var_elsewhere(annotation: &Expr, diagnostics: &mut Diagnostics) -> Type {
    diagnostics.push(
        Diagnostic::new(
            DiagnosticCode::error(408),
            annotation.span(),
            "`ClassVar` declares a class variable in a class body, given a value there",
        )
        .with_rule(RuleId::CLS_11),
    );
    Type::Error
}

/// The annotation of a class variable (CLS-11).
pub(crate) enum ClassVarAnnotation<'tree> {
    /// `ClassVar[T]`, with the annotation of its type.
    Typed(&'tree Expr),
    /// `ClassVar`, whose type its value gives.
    Bare,
}

/// Returns the annotation of a class variable written `ClassVar[T]` or `ClassVar`, or `None`
/// for any other annotation (CLS-11).
pub(crate) fn class_var_type<'tree>(
    annotation: &'tree Expr,
    names: &BTreeMap<String, AnnotationName>,
    standard: &BTreeMap<String, String>,
) -> Option<ClassVarAnnotation<'tree>> {
    qualified(annotation, AnnotationName::ClassVar, names, standard)
}

/// Returns the annotation of a name declared `Final[T]` or `Final`, or `None` for any other
/// annotation (TYP-08).
pub(crate) fn final_type<'tree>(
    annotation: &'tree Expr,
    names: &BTreeMap<String, AnnotationName>,
    standard: &BTreeMap<String, String>,
) -> Option<ClassVarAnnotation<'tree>> {
    qualified(annotation, AnnotationName::Final, names, standard)
}

/// Returns the annotation inside a qualifier of `typing`, `ClassVar` or `Final`, written with a
/// type or bare, or `None` when the annotation is not one.
fn qualified<'tree>(
    annotation: &'tree Expr,
    qualifier: AnnotationName,
    names: &BTreeMap<String, AnnotationName>,
    standard: &BTreeMap<String, String>,
) -> Option<ClassVarAnnotation<'tree>> {
    let word = match qualifier {
        AnnotationName::Final => "Final",
        _ => "ClassVar",
    };
    let is_qualifier = |expression: &Expr| match expression {
        Expr::Name { id, .. } => names.get(id) == Some(&qualifier),
        Expr::Attribute {
            value, attribute, ..
        } => {
            matches!(value.as_ref(), Expr::Name { id, .. }
                if standard.get(id).map(String::as_str) == Some("typing"))
                && attribute.name == word
        }
        _ => false,
    };
    match annotation {
        Expr::Subscript { value, index, .. } if is_qualifier(value) => {
            Some(ClassVarAnnotation::Typed(index))
        }
        other if is_qualifier(other) => Some(ClassVarAnnotation::Bare),
        _ => None,
    }
}

/// Returns the type `module.Class` names, through the name `id` bound to a module of the
/// program whose classes are `classes` (MOD-10).
fn module_class_type(
    id: &str,
    classes: &BTreeMap<String, ClassId>,
    attribute: &typy_source::syntax::Identifier,
    span: typy_diagnostics::Span,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    let only = BTreeMap::from([(
        attribute.name.clone(),
        classes
            .get(&attribute.name)
            .copied()
            .unwrap_or(ClassId(u32::MAX)),
    )]);
    if classes.contains_key(&attribute.name) {
        let inner = AnnotationScope {
            classes: &only,
            bound: u32::MAX,
            ..*scope
        };
        return named_type(&attribute.name, span, &inner, diagnostics);
    }
    diagnostics.push(
        Diagnostic::new(
            DiagnosticCode::error(425),
            attribute.span,
            format!("the module `{id}` has no class `{}`", attribute.name),
        )
        .with_rule(RuleId::SMT_07),
    );
    Type::Error
}

/// Returns the alias of another module an annotation names, by an imported name or through
/// its module, alone or given its type arguments.
fn imported_alias<'scope>(
    expression: &Expr,
    scope: &AnnotationScope<'scope>,
) -> Option<(String, &'scope ExportedAlias)> {
    match expression {
        Expr::Name { id, .. } => scope
            .imported_aliases
            .get(id)
            .map(|alias| (id.clone(), alias.as_ref())),
        Expr::Attribute {
            value, attribute, ..
        } => {
            let Expr::Name { id, .. } = value.as_ref() else {
                return None;
            };
            let alias = scope.module_aliases.get(id)?.get(&attribute.name)?;
            Some((format!("{id}.{}", attribute.name), alias.as_ref()))
        }
        _ => None,
    }
}

/// Returns the type an alias of another module stands for, its value read in its own module
/// with each type argument, read here, in place of its parameter (SMT-12).
fn imported_alias_type(
    annotation: &Expr,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Option<Type> {
    let (named, arguments) = match annotation {
        Expr::Subscript { value, index, .. } => {
            let found = imported_alias(value, scope)?;
            let arguments = match index.as_ref() {
                Expr::Tuple { elements, .. } => elements.iter().collect(),
                one => vec![one],
            };
            (found, arguments)
        }
        other => (imported_alias(other, scope)?, Vec::new()),
    };
    let (name, alias) = named;
    if arguments.len() != alias.parameters.len() {
        let count = alias.parameters.len();
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(408),
                annotation.span(),
                format!(
                    "the type alias `{name}` takes {count} type {}, such as `{name}[{}]`",
                    if count == 1 { "argument" } else { "arguments" },
                    vec!["int"; count].join(", ")
                ),
            )
            .with_rule(RuleId::SMT_10),
        );
        return Some(Type::Error);
    }
    let parameters: BTreeMap<String, Type> = alias
        .parameters
        .iter()
        .cloned()
        .zip(
            arguments
                .into_iter()
                .map(|argument| annotation_type(argument, scope, diagnostics)),
        )
        .collect();
    let written = &alias.names;
    let own = AnnotationScope {
        classes: &alias.classes,
        instances: scope.instances,
        modules: &written.modules,
        module_aliases: &written.module_aliases,
        imported_aliases: &written.aliases,
        names: &written.names,
        standard: &written.standard,
        parameters: &parameters,
        copies: scope.copies,
        bound: u32::MAX,
    };
    Some(annotation_type(&alias.value, &own, diagnostics))
}

/// Returns the standard module a name, or a dotted name through one, stands for.
fn standard_module(expression: &Expr, scope: &AnnotationScope<'_>) -> Option<String> {
    match expression {
        Expr::Name { id, .. } => scope.standard.get(id).cloned(),
        Expr::Attribute {
            value, attribute, ..
        } => Some(format!(
            "{}.{}",
            standard_module(value, scope)?,
            attribute.name
        )),
        _ => None,
    }
}

/// Returns the type an annotation names.
pub(crate) fn annotation_type(
    annotation: &Expr,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    if let Some(ty) = standard_annotation(annotation, scope, diagnostics) {
        return ty;
    }
    if let Some(ty) = imported_alias_type(annotation, scope, diagnostics) {
        return ty;
    }
    if let Some(ty) = generic_class_annotation(annotation, scope, diagnostics) {
        return ty;
    }
    match annotation {
        Expr::Constant {
            value: Constant::None,
            ..
        } => Type::NoneType,
        Expr::Name { id, span } => named_type(id, *span, scope, diagnostics),
        Expr::Attribute {
            value,
            attribute,
            span,
        } if let Expr::Name { id, .. } = value.as_ref()
            && let Some(classes) = scope.modules.get(id) =>
        {
            module_class_type(id, classes, attribute, *span, scope, diagnostics)
        }
        // A string is read when the annotation is, so it may name any class (CLS-01).
        Expr::Quoted { annotation, .. } => {
            let deferred = AnnotationScope {
                bound: u32::MAX,
                ..*scope
            };
            annotation_type(annotation, &deferred, diagnostics)
        }
        Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "tuple") => {
            tuple_annotation(index, scope, diagnostics)
        }
        Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "type") =>
        {
            let of = annotation_type(index, scope, diagnostics);
            crate::types::TypeObjectOf::type_of(crate::types::TypeObjectOf::of(&of))
        }
        Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "list") => {
            Type::List(Box::new(annotation_type(index, scope, diagnostics)))
        }
        Expr::BinOp {
            operator: typy_source::syntax::BinaryOperator::BitOr,
            left,
            right,
            ..
        } => {
            let left = annotation_type(left, scope, diagnostics);
            let right = annotation_type(right, scope, diagnostics);
            Type::union(vec![left, right])
        }
        Expr::Subscript { value, index, span } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "dict") => {
            dict_annotation(index, *span, scope, diagnostics)
        }
        Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if id == "set") =>
        {
            let item = annotation_type(index, scope, diagnostics);
            if !item.is_hashable() {
                diagnostics.push(unhashable(&item, index.span()));
                return Type::Error;
            }
            Type::Set(Box::new(item))
        }
        other => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(205),
                    other.span(),
                    "this annotation does not name a type",
                )
                .with_rule(RuleId::STD_04),
            );
            Type::Error
        }
    }
}

/// Returns the copy of a generic class an annotation names with its type arguments, the
/// class written by its name or through its module, or `None` when the annotation names
/// none (GEN-01, GEN-07, ADR-0028).
fn generic_class_annotation(
    annotation: &Expr,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Option<Type> {
    let Expr::Subscript { value, index, .. } = annotation else {
        return None;
    };
    let (name, class) = match value.as_ref() {
        Expr::Name { id, .. } => (id, scope.classes.get(id)?),
        Expr::Attribute {
            value: module,
            attribute,
            ..
        } => {
            let Expr::Name { id, .. } = module.as_ref() else {
                return None;
            };
            let class = scope.modules.get(id)?.get(&attribute.name)?;
            (&attribute.name, class)
        }
        _ => return None,
    };
    if !scope.copies.borrow().roots.contains_key(class) {
        return None;
    }
    Some(super::generic_class::applied_annotation(
        *class,
        name,
        index,
        scope,
        diagnostics,
    ))
}

/// Returns the type a bare name names: a scalar, or an error for a name that is not a type or
/// needs its item types.
fn named_type(
    id: &str,
    span: typy_diagnostics::Span,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    if let Some(parameter) = scope.parameters.get(id) {
        return parameter.clone();
    }
    if let Some(class) = scope.classes.get(id) {
        if scope.copies.borrow().roots.contains_key(class) {
            return super::generic_class::bare_annotation(*class, id, span, scope, diagnostics);
        }
        if class.0 >= scope.bound {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(306),
                    span,
                    format!("this annotation is evaluated before the `class {id}` statement runs"),
                )
                .with_rule(RuleId::TYP_12)
                .with_help(format!("write the annotation as a string, `\"{id}\"`")),
            );
            return Type::Error;
        }
        return scope
            .instances
            .get(class)
            .cloned()
            .unwrap_or_else(|| Type::instance(crate::class_type::InstanceClass::root(*class, id)));
    }
    match id {
        "int" => Type::Int,
        // `type` holds any type object; `type[X]` the one of `X` (CLS-12).
        "type" => Type::TypeObject(None),
        "float" => Type::Float,
        "bool" => Type::Bool,
        "str" => Type::Str,
        "bytes" => Type::Bytes,
        "bytearray" => Type::Bytearray,
        "range" => Type::Range,
        "slice" => Type::Slice,
        // A value of any type, narrowed before use, as Java holds `Object` (TYP-23, ADR-0090).
        "object" => Type::Object,
        "Any" => planned_open_type(id, span, diagnostics),
        "tuple" => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(408),
                    span,
                    "a `tuple` annotation names the type of each item, such as `tuple[int, str]`",
                )
                .with_rule(RuleId::TYP_06),
            );
            Type::Error
        }
        "list" => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(408),
                    span,
                    "a `list` annotation names the type of its items, such as `list[int]`",
                )
                .with_rule(RuleId::TYP_06),
            );
            Type::Error
        }
        "set" => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(408),
                    span,
                    "a `set` annotation names the type of its items, such as `set[int]`",
                )
                .with_rule(RuleId::TYP_06),
            );
            Type::Error
        }
        "dict" => {
            diagnostics.push(dict_arity(span));
            Type::Error
        }
        _ if let Some(class) = BuiltinException::named(id) => {
            Type::Exception(ExceptionClass::Builtin(class))
        }
        _ => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(205),
                    span,
                    format!(
                        "`{id}` names no type: it is not a built-in type, nor a class or a \
                         type alias the program defines or imports"
                    ),
                )
                .with_rule(RuleId::STD_04),
            );
            Type::Error
        }
    }
}

/// Returns the type `dict[K, V]` names: two types, and a key type with a hash.
fn dict_annotation(
    index: &Expr,
    span: typy_diagnostics::Span,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    let Expr::Tuple { elements, .. } = index else {
        diagnostics.push(dict_arity(span));
        return Type::Error;
    };
    let [key, value] = elements.as_slice() else {
        diagnostics.push(dict_arity(span));
        return Type::Error;
    };
    let (key_type, value_type) = (
        annotation_type(key, scope, diagnostics),
        annotation_type(value, scope, diagnostics),
    );
    if !key_type.is_hashable() {
        diagnostics.push(unhashable_key(&key_type, key.span()));
        return Type::Error;
    }
    Type::Dict(Box::new(key_type), Box::new(value_type))
}

fn dict_arity(span: typy_diagnostics::Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(408),
        span,
        "a `dict` annotation names the types of its keys and values, such as `dict[str, int]`",
    )
    .with_rule(RuleId::TYP_06)
}

/// Returns the error for an item of a set, or a key of a dict, whose type has no hash.
pub(crate) fn unhashable(item: &Type, span: typy_diagnostics::Span) -> Diagnostic {
    unhashable_as(item, span, "an item of a set", RuleId::DAT_10)
}

/// Returns the error for a key of a dict whose type has no hash.
pub(crate) fn unhashable_key(key: &Type, span: typy_diagnostics::Span) -> Diagnostic {
    unhashable_as(key, span, "a key of a dict", RuleId::DAT_09)
}

fn unhashable_as(
    item: &Type,
    span: typy_diagnostics::Span,
    place: &str,
    rule: RuleId,
) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(413),
        span,
        format!(
            "{} cannot be {place}, because it has no hash",
            item.with_article()
        ),
    )
    .with_rule(rule)
    .with_help("a list, a set or a dict changes, so it has no hash; use a tuple of its items")
}

/// Returns the type `tuple[...]` names: one type per item, `tuple[()]` for the empty tuple. A
/// tuple of any length, `tuple[T, ...]`, is a sequence like a list and arrives with lists.
fn tuple_annotation(
    index: &Expr,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    let items: Vec<&Expr> = match index {
        Expr::Tuple { elements, .. } => elements.iter().collect(),
        single => vec![single],
    };
    let is_ellipsis = |item: &Expr| {
        matches!(
            item,
            Expr::Constant {
                value: Constant::Ellipsis,
                ..
            }
        )
    };
    if let [item, ellipsis] = items.as_slice()
        && is_ellipsis(ellipsis)
        && !is_ellipsis(item)
    {
        return Type::TupleOf(Box::new(annotation_type(item, scope, diagnostics)));
    }
    if items.iter().any(|item| is_ellipsis(item)) {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(408),
                index.span(),
                "`...` in a tuple annotation follows its only item type, as in `tuple[int, ...]`",
            )
            .with_rule(RuleId::DAT_14),
        );
        return Type::Error;
    }
    Type::Tuple(
        items
            .into_iter()
            .map(|item| annotation_type(item, scope, diagnostics))
            .collect(),
    )
}

/// Returns the error type after reporting `Any` as a type, which a later release gives, bound
/// when the program runs (ADR-0094).
fn planned_open_type(
    id: &str,
    span: typy_diagnostics::Span,
    diagnostics: &mut Diagnostics,
) -> Type {
    let (release, rule) = ("0.13", RuleId::TYP_25);
    diagnostics.push(
        Diagnostic::new(
            DiagnosticCode::error(201),
            span,
            format!("`{id}` as a type is planned for TyPy {release} ({rule})"),
        )
        .with_rule(rule)
        .with_help("annotate the value with its type, or a union of its types"),
    );
    Type::Error
}

/// Returns the error type after reporting a name of `dataclasses` or `typing.cast` used as a
/// type: they are functions.
fn not_a_type(
    name: &AnnotationName,
    span: typy_diagnostics::Span,
    diagnostics: &mut Diagnostics,
) -> Type {
    let (message, rule) = if *name == AnnotationName::Cast {
        ("`cast` is a function, not a type", RuleId::TYP_24)
    } else {
        ("`dataclass` and `field` are not types", RuleId::CLS_05)
    };
    diagnostics.push(Diagnostic::new(DiagnosticCode::error(408), span, message).with_rule(rule));
    Type::Error
}
