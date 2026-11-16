//! Module-level classes: their names, their bases, the attributes they declare or `__init__`
//! assigns, and their methods (CLS-01, CLS-02, CLS-03, CLS-13).

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Identifier, Stmt};

use super::annotation::{AnnotationName, AnnotationScope, annotation_type, is_protocol};
use super::definition::{
    Definition, FunctionParts, MethodKind, Owner, defined_twice, definition,
    resolve_type_parameters,
};
use super::generic_class::finish_generic_class;
use super::overriding::Override;
use crate::class_type::{InstanceClass, Operators};
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::{ClassId, FunctionId, FunctionSignature, SignatureParameter};
use crate::types::{Type, TypeObjectOf};

/// An attribute of a class: declared in the class body, or assigned in `__init__`, whose
/// check gives it its type.
#[derive(Clone, Debug)]
pub(crate) struct AttributeInfo {
    pub(crate) name: String,
    /// The type; unknown until `__init__` assigns an attribute the class body does not declare.
    pub(crate) ty: Option<Type>,
}

/// A class while the program is checked.
#[derive(Clone, Debug)]
pub(crate) struct ClassInfo {
    pub(crate) id: ClassId,
    pub(crate) name: String,
    /// The base when it is a class of the program, whose attributes come first.
    pub(crate) base: Option<ClassId>,
    /// How many of the attributes come from the base.
    pub(crate) inherited: usize,
    /// The class as an exception class, when it derives from one (CLS-03).
    pub(crate) exception: Option<ExceptionClass>,
    pub(crate) attributes: Vec<AttributeInfo>,
    pub(crate) methods: Vec<(String, FunctionId)>,
    pub(crate) init: Option<FunctionId>,
    /// The class variables the class body gives a value, each held as the module variable
    /// `Class.name` (CLS-11, ADR-0024).
    pub(crate) class_variables: Vec<String>,
    /// The copies of the class methods the class has, each by its name (ADR-0025).
    pub(crate) class_methods: Vec<(String, FunctionId)>,
    /// The static methods the class defines (CLS-06).
    pub(crate) static_methods: Vec<String>,
    /// The methods with type parameters the class defines, each called through its copies
    /// rather than the class's method table (GEN-01).
    pub(crate) generic_methods: Vec<String>,
    /// The properties the class defines, each a getter among the methods, and its setter, when
    /// it has one, as the method `name.setter` (CLS-06).
    pub(crate) properties: Vec<String>,
    /// The positions of the class's own attributes the class body gives a default, each held
    /// as the module variable `Class.name`.
    /// Each method that overrides one held otherwise in a base, by its name, with the base
    /// method and itself, which a call through the base reaches through an adapter (CLS-04).
    pub(crate) adapted: Vec<(String, FunctionId, FunctionId)>,
    /// The adapter of each such method, by its name.
    pub(crate) adapters: Vec<(String, FunctionId)>,
    /// The bridge of each static method of the class's own, by its name: a method that takes
    /// the instance first, which the method table holds in the static method's place (CLS-06).
    pub(crate) static_bridges: Vec<(String, FunctionId)>,
    pub(crate) defaults: Vec<usize>,
    /// Whether the class is a frozen dataclass, whose fields only the `__init__` `@dataclass`
    /// writes assigns (CLS-05).
    pub(crate) frozen: bool,
    /// Whether `@dataclass` wrote the class's `__init__`.
    pub(crate) generated_init: bool,
    /// What the class body assigns to `__match_args__`, which a class pattern's positional
    /// sub-patterns name: the names of a tuple of `str` literals, or where a value of another
    /// form is assigned; `None` when the body assigns none, so the base's apply (CST-06).
    pub(crate) match_args: Option<Result<Vec<String>, Span>>,
    /// Whether the class's `__init__` uses the instance other than through its attributes,
    /// once checked.
    pub(crate) init_uses_instance: bool,
    /// The attributes `__init__` declares `Final`, each by its position with the statement
    /// that alone assigns it (TYP-08).
    pub(crate) final_attributes: Vec<(usize, Span)>,
    pub(crate) span: Span,
}

impl ClassInfo {
    /// Returns the position of an attribute.
    pub(crate) fn attribute(&self, name: &str) -> Option<usize> {
        self.attributes
            .iter()
            .position(|attribute| attribute.name == name)
    }

    /// Returns a method by its name.
    pub(crate) fn method(&self, name: &str) -> Option<FunctionId> {
        self.methods
            .iter()
            .find(|(each, _)| each == name)
            .map(|(_, id)| *id)
    }
}

/// A `class` statement, with its ID in source order.
pub(super) struct ClassStatement<'tree> {
    pub(super) id: ClassId,
    pub(super) name: &'tree Identifier,
    pub(super) base: Option<&'tree Expr>,
    pub(super) body: &'tree [Stmt],
    /// The type parameters of a generic class (GEN-01).
    pub(super) type_parameters: &'tree [typy_source::syntax::TypeParameter],
    pub(super) span: Span,
}

/// Collects the module's classes in source order, numbered from `first`, the number of
/// classes of the modules checked before it.
pub(super) fn collect_classes<'tree>(
    statements: &'tree [Stmt],
    first: usize,
    diagnostics: &mut Diagnostics,
) -> Vec<ClassStatement<'tree>> {
    let mut classes: Vec<ClassStatement<'tree>> = Vec::new();
    for statement in statements {
        let Stmt::ClassDef {
            name,
            bases,
            body,
            type_parameters,
            span,
            ..
        } = statement
        else {
            continue;
        };
        if classes.iter().any(|class| class.name.name == name.name) {
            diagnostics.push(defined_twice(name));
            continue;
        }
        classes.push(ClassStatement {
            id: ClassId(u32::try_from(first + classes.len()).unwrap_or(u32::MAX)),
            name,
            base: bases.first(),
            body,
            type_parameters,
            span: *span,
        });
    }
    classes
}

/// The classes a base may name: those of the modules checked before, by ID, with the type of
/// their instances, and the names an annotation may use for them.
pub(super) struct BaseScope<'names> {
    pub(super) known: &'names BTreeMap<ClassId, Type>,
    pub(super) class_ids: &'names BTreeMap<String, ClassId>,
    pub(super) modules: &'names BTreeMap<String, BTreeMap<String, ClassId>>,
    /// The names of `typing` and the modules imported, which name `Protocol` (GEN-04).
    pub(super) names: &'names BTreeMap<String, AnnotationName>,
    pub(super) standard: &'names BTreeMap<String, String>,
}

/// Resolves each class's base, in source order, and returns the type of each class's
/// instances: an exception for a class derived from a built-in exception class or from a
/// class of the program that is one (CLS-03), otherwise an instance whose chain holds the
/// classes of the program it derives from (CLS-04, ADR-0021). A base that cannot be resolved
/// is reported, and the class derives from `object`, so its uses are still checked.
pub(super) fn class_types(
    classes: &[ClassStatement<'_>],
    scope: &BaseScope<'_>,
    diagnostics: &mut Diagnostics,
) -> BTreeMap<ClassId, Type> {
    let mut found: BTreeMap<ClassId, Type> = BTreeMap::new();
    for class in classes {
        let name = class.name.name.clone();
        let protocol = class
            .base
            .is_some_and(|base| is_protocol(base, scope.names, scope.standard));
        let ty = class
            .base
            .filter(|_| !protocol)
            .and_then(|base| derived_type(class, base, scope, &found, diagnostics))
            .unwrap_or_else(|| {
                Type::instance(InstanceClass {
                    operators: Operators::of(method_names(class.body), Operators::default()),
                    protocol,
                    ..InstanceClass::root(class.id, name)
                })
            });
        if !class.type_parameters.is_empty() {
            refuse_generic_kind(class, &ty, diagnostics);
        }
        found.insert(class.id, ty);
    }
    found
}

/// Reports a generic class that is an exception class or a protocol, whose copies would need
/// exception objects or protocol checks of their own (GEN-12, GEN-13).
fn refuse_generic_kind(class: &ClassStatement<'_>, ty: &Type, diagnostics: &mut Diagnostics) {
    let name = &class.name.name;
    let (what, rule) = match ty {
        Type::Exception(_) => ("a generic exception class", RuleId::GEN_12),
        Type::Instance(instance) if instance.protocol => ("a generic protocol", RuleId::GEN_13),
        _ => return,
    };
    diagnostics.push(
        Diagnostic::new(
            DiagnosticCode::error(201),
            class.name.span,
            format!("`{name}` is {what}, which is planned for TyPy 1.0 ({rule})"),
        )
        .with_rule(rule),
    );
}

/// Returns the type of the instances of a class with a base other than `object`, or `None`
/// for `object` and for a base that is reported.
fn derived_type(
    class: &ClassStatement<'_>,
    base: &Expr,
    scope: &BaseScope<'_>,
    found: &BTreeMap<ClassId, Type>,
    diagnostics: &mut Diagnostics,
) -> Option<Type> {
    let name = class.name.name.clone();
    let span = base.span();
    let (written, base_id) = match base {
        Expr::Name { id, .. } if id == "object" => return None,
        Expr::Name { id, .. } => {
            if let Some(builtin) = BuiltinException::named(id) {
                if !builtin.can_be_called() {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(417),
                            span,
                            format!("a class cannot derive from `{id}` in TyPy"),
                        )
                        .with_rule(RuleId::EXC_12)
                        .with_help("derive from `ValueError` instead"),
                    );
                    return None;
                }
                return Some(Type::Exception(ExceptionClass::program(
                    class.id,
                    name,
                    Vec::new(),
                    builtin,
                )));
            }
            (id.clone(), scope.class_ids.get(id).copied())
        }
        Expr::Attribute {
            value, attribute, ..
        } => match value.as_ref() {
            Expr::Name { id, .. } => (
                format!("{id}.{}", attribute.name),
                scope
                    .modules
                    .get(id)
                    .and_then(|classes| classes.get(&attribute.name))
                    .copied(),
            ),
            _ => (String::new(), None),
        },
        _ => (String::new(), None),
    };
    let Some(base_id) = base_id else {
        diagnostics.push(unsupported_base(span));
        return None;
    };
    if base_id >= class.id {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(306),
                span,
                format!("`{written}` is defined after the class that derives from it"),
            )
            .with_rule(RuleId::TYP_12),
        );
        return None;
    }
    match found.get(&base_id).or_else(|| scope.known.get(&base_id))? {
        Type::Exception(ExceptionClass::Class(parent)) => {
            let mut chain = vec![(base_id, parent.name.clone())];
            chain.extend(parent.ancestors.iter().cloned());
            Some(Type::Exception(ExceptionClass::program(
                class.id,
                name,
                chain,
                parent.builtin,
            )))
        }
        // A class satisfies a protocol by its members; naming the protocol as its base
        // would also inherit the protocol's methods (GEN-10).
        Type::Instance(parent) if parent.protocol => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    format!(
                        "a class derived from the protocol `{written}` is planned for TyPy 0.5 \
                         (GEN-10)"
                    ),
                )
                .with_rule(RuleId::GEN_10)
                .with_help("leave the protocol out: a class with its members satisfies it"),
            );
            None
        }
        Type::Instance(parent) => {
            let mut ancestors = vec![(base_id, parent.name.clone())];
            ancestors.extend(parent.ancestors.iter().cloned());
            Some(Type::instance(InstanceClass {
                id: class.id,
                name,
                ancestors,
                operators: Operators::of(method_names(class.body), parent.operators),
                generic: None,
                protocol: false,
            }))
        }
        _ => None,
    }
}

/// Returns the names of the methods a class body defines.
fn method_names(body: &[Stmt]) -> impl Iterator<Item = &str> {
    body.iter().filter_map(|statement| match statement {
        Stmt::FunctionDef { name, .. } => Some(name.name.as_str()),
        _ => None,
    })
}

/// Returns the error for a base other than `object`, an exception class or a class of the
/// program: a built-in type such as `list`, or an expression that names no class.
fn unsupported_base(span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        "a class derives from `object`, an exception class or a class of the program in TyPy \
         (CLS-04)",
    )
    .with_rule(RuleId::CLS_04)
}

/// The special methods a class may define: `__init__`, `__str__` and `__repr__`, and from 0.3
/// `__iter__` and `__next__` (CST-08) and `__enter__` and `__exit__` (CST-05). Any other name that starts and ends with two
/// underscores changes how Python treats the instances, and waits for the operator methods of
/// 0.4 (CLS-07).
const SPECIAL_METHODS: [&str; 63] = [
    "__init__",
    "__str__",
    "__repr__",
    "__iter__",
    "__next__",
    "__enter__",
    "__exit__",
    "__eq__",
    "__ne__",
    "__lt__",
    "__le__",
    "__gt__",
    "__ge__",
    "__hash__",
    "__add__",
    "__radd__",
    "__sub__",
    "__rsub__",
    "__mul__",
    "__rmul__",
    "__matmul__",
    "__rmatmul__",
    "__truediv__",
    "__rtruediv__",
    "__floordiv__",
    "__rfloordiv__",
    "__mod__",
    "__rmod__",
    "__pow__",
    "__rpow__",
    "__lshift__",
    "__rlshift__",
    "__rshift__",
    "__rrshift__",
    "__and__",
    "__rand__",
    "__or__",
    "__ror__",
    "__xor__",
    "__rxor__",
    "__neg__",
    "__pos__",
    "__invert__",
    "__abs__",
    "__len__",
    "__getitem__",
    "__setitem__",
    "__delitem__",
    "__contains__",
    "__bool__",
    "__iadd__",
    "__isub__",
    "__imul__",
    "__imatmul__",
    "__itruediv__",
    "__ifloordiv__",
    "__imod__",
    "__ipow__",
    "__ilshift__",
    "__irshift__",
    "__iand__",
    "__ior__",
    "__ixor__",
];

/// The classes a module's class bodies may name: each by its name, the type of each one's
/// instances, and the classes of the modules checked before, indexed by ID.
pub(super) struct KnownClasses<'known> {
    pub(super) ids: &'known BTreeMap<String, ClassId>,
    pub(super) instances: &'known BTreeMap<ClassId, Type>,
    pub(super) earlier: &'known [ClassInfo],
    /// The signatures of the functions of the modules checked before, indexed by ID.
    pub(super) signatures: &'known [FunctionSignature],
    /// The generic classes and their copies (ADR-0028).
    pub(super) copies: &'known std::cell::RefCell<crate::body::ClassCopies>,
    /// The methods that return `T | NotImplementedType` (CLS-21).
    pub(super) giving_way: &'known super::not_implemented::GivingWay,
}

/// Collects each class's attributes and methods; the methods become definitions whose IDs
/// follow `first_function`.
pub(super) fn collect_members<'tree>(
    known: &KnownClasses<'_>,
    classes: &[ClassStatement<'tree>],
    names: &crate::check::ImportedNames,
    floor_defers: bool,
    (first_function, copies): (usize, &mut ClassMethodCopies<'tree>),
    diagnostics: &mut Diagnostics,
) -> (Vec<ClassInfo>, Vec<Definition<'tree>>) {
    let mut infos = Vec::new();
    let mut methods: Vec<Definition<'tree>> = Vec::new();
    for class in classes {
        let mut own_class_methods: Vec<&'tree Stmt> = Vec::new();
        let in_scope = super::generic_class::parameters_in_scope(known.copies, class.id);
        let scope = class_scope(known, names, floor_defers, class.id, &in_scope);
        let first_method = methods.len();
        let instance = class_instance(known, class);
        let classes = Classes {
            earlier: known.earlier,
            local: &infos,
        };
        let mut info = new_info(class, &instance, classes);
        let owner = Owner {
            id: class.id,
            instance,
            takes_instance: true,
        };
        for statement in class.body {
            match (statement, FunctionParts::of(statement)) {
                (Stmt::AnnAssign { .. } | Stmt::Assign { .. }, _) => {
                    class_body_name(statement, &mut info, &scope, names, diagnostics);
                }
                (_, Some(parts)) => {
                    let name = &parts.name.name;
                    let kind = parts.method_kind();
                    let registered = registered_name(&kind, name);
                    if let Some(refused) = refused_method(&parts, &kind, &registered, &info)
                        .or_else(|| super::protocol::refused_member(&owner.instance, &kind, &parts))
                    {
                        diagnostics.push(refused);
                        continue;
                    }
                    if kind == MethodKind::Class {
                        own_class_methods.push(statement);
                        continue;
                    }
                    let id = FunctionId(
                        u32::try_from(first_function + methods.len()).unwrap_or(u32::MAX),
                    );
                    // A special method takes no decorator, so `__init__` is an instance method.
                    if name == "__init__" {
                        note_init(&parts, id, &mut info, names);
                    }
                    let mut method = method_definition(
                        &parts,
                        &kind,
                        id,
                        &mut info,
                        &owner,
                        &scope,
                        diagnostics,
                    );
                    give_way(&mut method, (known, class.span), parts.name, diagnostics);
                    check_accessor(
                        &kind,
                        &method,
                        &info,
                        &methods,
                        first_function,
                        parts.name,
                        diagnostics,
                    );
                    let signatures = Signatures {
                        earlier: known.signatures,
                        local: &methods,
                        first_function,
                    };
                    let override_of = (&method, kind == MethodKind::Static);
                    match check_override(&info, classes, signatures, override_of, parts.name) {
                        Err(diagnostic) => diagnostics.push(*diagnostic),
                        // A call through the base reaches the override through an adapter.
                        Ok(Some(base)) => info.adapted.push((name.clone(), base, id)),
                        Ok(None) => {}
                    }
                    check_special_method(&method, parts.name, diagnostics);
                    methods.push(method);
                }
                _ => {}
            }
        }
        let mut collected = Collected {
            methods: &mut methods,
            first_function,
        };
        copies.make(
            &own_class_methods,
            &mut info,
            (known, &scope),
            &mut collected,
            diagnostics,
        );
        report_name_conflicts(&info, class.name.span, diagnostics);
        super::protocol::check_values(&owner.instance, &info, class.name.span, diagnostics);
        let own_methods = &mut methods[first_method..];
        finish_generic_class(&info, own_methods, known.copies, diagnostics);
        infos.push(info);
    }
    copies.end_module(known.ids, names);
    (infos, methods)
}

/// Returns the type of a class's instances, as the class types gave it.
fn class_instance(known: &KnownClasses<'_>, class: &ClassStatement<'_>) -> Type {
    known
        .instances
        .get(&class.id)
        .cloned()
        .unwrap_or_else(|| Type::instance(InstanceClass::root(class.id, &class.name.name)))
}

/// The classes collected so far: those of the modules checked before, indexed by ID, and the
/// module's own classes before the one being collected.
#[derive(Clone, Copy)]
struct Classes<'infos> {
    earlier: &'infos [ClassInfo],
    local: &'infos [ClassInfo],
}

impl<'infos> Classes<'infos> {
    /// Returns a class collected before; a base is defined before the class, so it is one.
    fn get(self, class: ClassId) -> Option<&'infos ClassInfo> {
        self.earlier
            .get(class.0 as usize)
            .or_else(|| self.local.iter().find(|info| info.id == class))
    }
}

/// Returns the class's information before its body is read: a class whose base is a class of
/// the program holds its base's attributes first, so each keeps its place (ADR-0021).
fn new_info(class: &ClassStatement<'_>, instance: &Type, classes: Classes<'_>) -> ClassInfo {
    let (base, exception) = match instance {
        Type::Exception(exception) => (
            match exception {
                ExceptionClass::Class(program) => program.ancestors.first().map(|(base, _)| *base),
                ExceptionClass::Builtin(_) => None,
            },
            Some(exception.clone()),
        ),
        Type::Instance(own) => (own.ancestors.first().map(|(base, _)| *base), None),
        _ => (None, None),
    };
    let attributes: Vec<AttributeInfo> = base
        .and_then(|base| classes.get(base))
        .map_or_else(Vec::new, |info| info.attributes.clone());
    ClassInfo {
        id: class.id,
        name: class.name.name.clone(),
        base,
        inherited: attributes.len(),
        exception,
        attributes,
        methods: Vec::new(),
        init: None,
        class_variables: Vec::new(),
        class_methods: Vec::new(),
        static_methods: Vec::new(),
        generic_methods: Vec::new(),
        properties: Vec::new(),
        defaults: Vec::new(),
        adapted: Vec::new(),
        adapters: Vec::new(),
        static_bridges: Vec::new(),
        frozen: false,
        generated_init: false,
        match_args: None,
        init_uses_instance: false,
        final_attributes: Vec::new(),
        span: class.span,
    }
}

/// Returns the classes an annotation in a class body may name; the class's own name is bound
/// only after its body runs.
fn class_scope<'known>(
    known: &KnownClasses<'known>,
    names: &'known crate::check::ImportedNames,
    floor_defers: bool,
    class: ClassId,
    parameters: &'known BTreeMap<String, Type>,
) -> AnnotationScope<'known> {
    AnnotationScope {
        classes: known.ids,
        instances: known.instances,
        modules: &names.modules,
        module_aliases: &names.module_aliases,
        imported_aliases: &names.aliases,
        parameters,
        copies: known.copies,
        names: &names.names,
        standard: &names.standard,
        bound: if floor_defers { u32::MAX } else { class.0 },
    }
}

/// Notes a class's `__init__`, and the attributes it assigns that the class body does not
/// declare.
fn note_init(
    parts: &FunctionParts<'_>,
    id: FunctionId,
    info: &mut ClassInfo,
    names: &crate::check::ImportedNames,
) {
    info.init = Some(id);
    for attribute in initialized_attributes(parts) {
        if info.attribute(&attribute).is_none() {
            info.attributes.push(AttributeInfo {
                name: attribute,
                ty: None,
            });
        }
    }
    let is_instance = |id: &str| {
        parts
            .parameters
            .first()
            .is_some_and(|instance| instance.name.name == id)
    };
    // Only a statement of the body itself declares one, so it runs once per instance.
    for statement in parts.body {
        if let Stmt::AnnAssign {
            target: Expr::Attribute {
                value, attribute, ..
            },
            annotation,
            value: Some(_),
            span,
            ..
        } = statement
            && matches!(value.as_ref(), Expr::Name { id, .. } if is_instance(id))
            && super::annotation::final_type(annotation, &names.names, &names.standard).is_some()
            && let Some(index) = info.attribute(&attribute.name)
        {
            info.final_attributes.push((index, *span));
        }
    }
}

/// The methods collected so far, whose IDs follow `first_function`.
struct Collected<'methods, 'tree> {
    methods: &'methods mut Vec<Definition<'tree>>,
    first_function: usize,
}

/// The class methods each class of the program has, its own and those it inherits, each with
/// the copy of the class that defines it and that class's module; and the names each module
/// checked so far gives its annotations, which a copy made in another module reads
/// (ADR-0025, ADR-0037).
#[derive(Default)]
pub(crate) struct ClassMethodCopies<'tree> {
    by_class: BTreeMap<ClassId, Vec<(&'tree Stmt, FunctionId, String)>>,
    names: BTreeMap<String, (BTreeMap<String, ClassId>, crate::check::ImportedNames)>,
    module: String,
}

impl<'tree> ClassMethodCopies<'tree> {
    /// Starts the copies of the module `module`.
    pub(crate) fn begin_module(&mut self, module: &str) {
        module.clone_into(&mut self.module);
    }

    /// Keeps the names of the module just collected, for copies its class methods get in
    /// modules checked after it.
    fn end_module(
        &mut self,
        classes: &BTreeMap<String, ClassId>,
        names: &crate::check::ImportedNames,
    ) {
        self.names
            .insert(self.module.clone(), (classes.clone(), names.clone()));
    }

    /// Makes a copy, for the class `info` describes, of each class method it has: its own, and
    /// each one a base defines that it does not define again. A copy of a class method of
    /// another module reads its signature in that module's names, and notes the module, whose
    /// names its body is checked in (ADR-0037).
    fn make(
        &mut self,
        own: &[&'tree Stmt],
        info: &mut ClassInfo,
        (known, scope): (&KnownClasses<'_>, &AnnotationScope<'_>),
        collected: &mut Collected<'_, 'tree>,
        diagnostics: &mut Diagnostics,
    ) {
        let name_of = |statement: &Stmt| match statement {
            Stmt::FunctionDef { name, .. } => name.name.clone(),
            _ => String::new(),
        };
        let own_names: Vec<String> = own.iter().map(|statement| name_of(statement)).collect();
        let mut visible: Vec<(&'tree Stmt, Option<FunctionId>, String)> = own
            .iter()
            .map(|statement| (*statement, None, self.module.clone()))
            .collect();
        if let Some(inherited) = info.base.and_then(|base| self.by_class.get(&base)) {
            visible.extend(
                inherited
                    .iter()
                    .filter(|(statement, _, _)| !own_names.contains(&name_of(statement)))
                    .map(|(statement, origin, module)| (*statement, Some(*origin), module.clone())),
            );
        }
        let class_object = match &info.exception {
            Some(exception) => TypeObjectOf::Exception(exception.clone()),
            None => TypeObjectOf::Instance(info.id, info.name.clone()),
        };
        let owner = Owner {
            id: info.id,
            instance: TypeObjectOf::type_of(Some(class_object)),
            takes_instance: true,
        };
        let mut made = Vec::new();
        // Each statement is a `def`, since only methods are copied.
        for (statement, parts, origin, module) in
            visible
                .into_iter()
                .filter_map(|(statement, origin, module)| {
                    Some((statement, FunctionParts::of(statement)?, origin, module))
                })
        {
            let id = FunctionId(
                u32::try_from(collected.first_function + collected.methods.len())
                    .unwrap_or(u32::MAX),
            );
            // An inherited copy repeats the signature of the copy it is made from, whose
            // errors are reported there.
            let mut repeated = Diagnostics::new();
            let sink = if origin.is_some() {
                &mut repeated
            } else {
                &mut *diagnostics
            };
            let home = (module != self.module).then(|| module.clone());
            let mut method = match home.as_ref().and_then(|home| self.names.get(home)) {
                // A copy of another module's class method reads its annotations there.
                Some((classes, names)) => {
                    let remote = AnnotationScope {
                        classes,
                        modules: &names.modules,
                        module_aliases: &names.module_aliases,
                        imported_aliases: &names.aliases,
                        names: &names.names,
                        standard: &names.standard,
                        bound: u32::MAX,
                        ..class_scope(known, names, true, info.id, scope.parameters)
                    };
                    definition(&parts, Some(&owner), &remote, sink)
                }
                None => definition(&parts, Some(&owner), scope, sink),
            };
            method.home = home;
            // A parameter that names a protocol would make the copy generic (GEN-09).
            let written = method.type_parameters.len();
            super::protocol::protocol_parameters(&mut method, sink);
            if method.type_parameters.len() > written {
                sink.push(planned_generic_method(
                    parts.name,
                    "a protocol parameter of a special method, a property or a class method is",
                ));
            }
            method.qualified_name = format!("{}.{}", info.name, parts.name.name);
            method.copy_of = origin;
            info.class_methods.push((parts.name.name.clone(), id));
            made.push((statement, origin.unwrap_or(id), module));
            collected.methods.push(method);
        }
        self.by_class.insert(info.id, made);
    }
}

/// Widens the return type of a method that may return `NotImplemented`, as the scan of the
/// program found, to take it (CLS-21).
fn give_way(
    method: &mut Definition<'_>,
    (known, class): (&KnownClasses<'_>, Span),
    name: &Identifier,
    diagnostics: &mut Diagnostics,
) {
    if !known.giving_way.contains(&(class, name.name.clone())) {
        return;
    }
    let declared = std::mem::replace(&mut method.signature.returns, Type::Error);
    if matches!(declared, Type::Optional(_) | Type::NoneType) {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(201),
                name.span,
                format!(
                    "`{}` returns `None` and `NotImplemented`, and an operator method that may \
                     return both is planned for TyPy 1.0 (CLS-33)",
                    name.name
                ),
            )
            .with_rule(RuleId::CLS_33),
        );
    }
    method.signature.returns = Type::union(vec![declared, Type::NotImplemented]);
}

/// Returns the name a method is held under: its own, or `name.setter` for the setter of the
/// property `name`, beside its getter, under a name no Python name can be (CLS-06).
fn registered_name(kind: &MethodKind, name: &str) -> String {
    match kind {
        MethodKind::Setter(property) => format!("{property}.setter"),
        _ => name.to_owned(),
    }
}

/// Records a method in its class, under the name it is held under, and returns its
/// definition; a static method's first parameter is not the instance.
fn method_definition<'tree>(
    parts: &FunctionParts<'tree>,
    kind: &MethodKind,
    id: FunctionId,
    info: &mut ClassInfo,
    owner: &Owner,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Definition<'tree> {
    let name = &parts.name.name;
    let registered = registered_name(kind, name);
    info.methods.push((registered.clone(), id));
    match kind {
        MethodKind::Static => info.static_methods.push(name.clone()),
        MethodKind::Getter => info.properties.push(name.clone()),
        _ => {}
    }
    let method_owner = Owner {
        id: owner.id,
        instance: owner.instance.clone(),
        takes_instance: *kind != MethodKind::Static,
    };
    // A method's annotations name its own type parameters and its class's.
    let type_parameters =
        resolve_type_parameters(parts.type_parameters, parts.span, scope, diagnostics);
    let mut in_scope = scope.parameters.clone();
    in_scope.extend(
        type_parameters
            .iter()
            .map(|parameter| (parameter.name.clone(), Type::parameter(parameter.clone()))),
    );
    let method_scope = AnnotationScope {
        parameters: &in_scope,
        ..*scope
    };
    let mut method = definition(parts, Some(&method_owner), &method_scope, diagnostics);
    method.type_parameters = type_parameters;
    let written = method.type_parameters.len();
    super::protocol::protocol_parameters(&mut method, diagnostics);
    // A parameter that names a protocol makes the method generic, which only an instance or
    // a static method may be yet (GEN-09).
    let special = name.starts_with("__") && name.ends_with("__");
    if method.type_parameters.len() > written
        && (special || !matches!(kind, MethodKind::Instance | MethodKind::Static))
    {
        diagnostics.push(planned_generic_method(
            parts.name,
            "a protocol parameter of a special method, a property or a class method is",
        ));
    }
    if !method.type_parameters.is_empty() {
        info.generic_methods.push(registered.clone());
    }
    if registered != *name {
        method.qualified_name = format!("{}.{registered}", info.name);
        method.name = registered;
    }
    method
}

/// Returns why a method cannot be defined, if it cannot: the class already defines it, it is
/// a special method planned with the operator methods, or it has no `self`.
fn refused_method(
    parts: &FunctionParts<'_>,
    kind: &MethodKind,
    registered: &str,
    info: &ClassInfo,
) -> Option<Diagnostic> {
    let name = &parts.name.name;
    let refuse = |code: u16, message: String| {
        Some(
            Diagnostic::new(DiagnosticCode::error(code), parts.name.span, message)
                .with_rule(RuleId::CLS_06),
        )
    };
    if info.method(registered).is_some() {
        return Some(defined_twice(parts.name));
    }
    let special = name.starts_with("__") && name.ends_with("__");
    if special && !SPECIAL_METHODS.contains(&name.as_str()) {
        return Some(planned_method(parts.name));
    }
    let generic = !parts.type_parameters.is_empty();
    if generic && (special || !matches!(kind, MethodKind::Instance | MethodKind::Static)) {
        return Some(planned_generic_method(
            parts.name,
            "type parameters of a special method, a property or a class method are",
        ));
    }
    if special && *kind != MethodKind::Instance {
        return refuse(
            404,
            format!("the special method `{name}` takes no decorator"),
        );
    }
    match kind {
        MethodKind::Class if parts.parameters.is_empty() => refuse(
            404,
            format!("the class method `{name}` takes the class, `cls`, first"),
        ),
        MethodKind::Static => None,
        MethodKind::Setter(property) if property != name => refuse(
            404,
            format!("the setter of the property `{property}` is a method named `{property}`"),
        ),
        MethodKind::Setter(property) if !info.properties.contains(property) => refuse(
            404,
            format!(
                "`@{property}.setter` follows the `@property` that defines `{property}` in the same class"
            ),
        ),
        _ if parts.parameters.is_empty() => Some(
            Diagnostic::new(
                DiagnosticCode::error(404),
                parts.name.span,
                format!("the method `{name}` takes the instance, `self`, first"),
            )
            .with_rule(RuleId::CLS_06)
            .with_help("add `self`, or decorate the function with `@staticmethod`"),
        ),
        _ => None,
    }
}

/// Checks the signature of a property's getter, which takes only the instance, and of its
/// setter, which takes the instance and a value of the getter's type and returns `None`.
fn check_accessor(
    kind: &MethodKind,
    method: &Definition<'_>,
    info: &ClassInfo,
    methods: &[Definition<'_>],
    first_function: usize,
    name: &Identifier,
    diagnostics: &mut Diagnostics,
) {
    let signature = &method.signature;
    let problem = match kind {
        MethodKind::Getter if signature.parameters.len() != 1 => {
            Some(format!("the property `{}` takes only `self`", name.name))
        }
        MethodKind::Setter(property) => {
            let getter = info
                .method(property)
                .and_then(|id| (id.0 as usize).checked_sub(first_function))
                .and_then(|local| methods.get(local))
                .map(|getter| getter.signature.returns.clone());
            let fits = signature.parameters.len() == 2
                && signature.returns.matches(&Type::NoneType)
                && getter.is_none_or(|ty| signature.parameters[1].ty.matches(&ty));
            (!fits).then(|| {
                format!(
                    "the setter of `{property}` takes `self` and a value of the property's type, \
                     and returns `None`"
                )
            })
        }
        _ => None,
    };
    if let Some(message) = problem {
        diagnostics.push(
            Diagnostic::new(DiagnosticCode::error(404), name.span, message)
                .with_rule(RuleId::CLS_06),
        );
    }
}

/// The signatures of the functions collected so far: those of the modules checked before,
/// indexed by ID, and the module's methods, whose IDs follow `first_function`.
#[derive(Clone, Copy)]
struct Signatures<'definitions, 'tree> {
    earlier: &'definitions [FunctionSignature],
    local: &'definitions [Definition<'tree>],
    first_function: usize,
}

impl<'definitions> Signatures<'definitions, '_> {
    fn get(self, id: FunctionId) -> Option<&'definitions FunctionSignature> {
        (id.0 as usize)
            .checked_sub(self.first_function)
            .map_or_else(
                || self.earlier.get(id.0 as usize),
                |local| self.local.get(local).map(|base| &base.signature),
            )
    }
}

/// Returns why a method does not fit the base method it overrides, if it overrides one and
/// does not fit; a constructor runs on its own class only, so `__init__` is not compared.
/// Checks a method against the base method it overrides, if it overrides one, and returns
/// the base method when a call through it needs an adapter to reach the override (CLS-04).
fn check_override(
    info: &ClassInfo,
    classes: Classes<'_>,
    signatures: Signatures<'_, '_>,
    (method, static_own): (&Definition<'_>, bool),
    name: &Identifier,
) -> Result<Option<FunctionId>, Box<Diagnostic>> {
    if name.name == "__init__" {
        return Ok(None);
    }
    let Some((base_id, base_class)) = inherited_method(classes, info.base, &name.name) else {
        return Ok(None);
    };
    // A copy of a generic method is called directly, so neither side of an override may be
    // generic.
    if !method.type_parameters.is_empty() || generic_method_in_bases(classes, info.base, name) {
        return Err(Box::new(planned_generic_method(
            name,
            "a method with type parameters that overrides or is overridden is",
        )));
    }
    let Some(base) = signatures.get(base_id) else {
        return Ok(None);
    };
    // A static method is reached through an instance by its bridge, which takes the instance
    // first, so it is compared as a method that takes one (CLS-06).
    let static_base = nearest_is_static(classes, info.base, &name.name);
    let as_method = |signature: &FunctionSignature, is_static: bool| {
        let mut signature = signature.clone();
        if is_static {
            signature.parameters.insert(
                0,
                SignatureParameter {
                    name: "self".to_owned(),
                    ty: Type::Error,
                    default: None,
                    default_call: None,
                },
            );
        }
        signature
    };
    let (own, base) = (
        as_method(&method.signature, static_own),
        as_method(base, static_base),
    );
    let fit = Override {
        name: &format!("{}.{}", info.name, name.name),
        signature: &own,
        base_name: &format!("{base_class}.{}", name.name),
        base: &base,
        span: name.span,
    }
    .check()?;
    if fit == super::overriding::Fit::Adapted && (static_own || static_base) {
        return Err(Box::new(
            Diagnostic::new(
                DiagnosticCode::error(201),
                name.span,
                format!(
                    "a static method that overrides `{base_class}.{}`, or is overridden, with a \
                     parameter or result held otherwise is planned for TyPy 1.0 (CLS-35)",
                    name.name
                ),
            )
            .with_rule(RuleId::CLS_35),
        ));
    }
    Ok((fit == super::overriding::Fit::Adapted).then_some(base_id))
}

/// Returns true when the nearest class from `base` up that defines `name` defines it as a
/// static method.
fn nearest_is_static(classes: Classes<'_>, base: Option<ClassId>, name: &str) -> bool {
    std::iter::successors(base.and_then(|class| classes.get(class)), |info| {
        info.base.and_then(|class| classes.get(class))
    })
    .find(|info| info.method(name).is_some())
    .is_some_and(|info| info.static_methods.iter().any(|each| each == name))
}

/// Returns true when the nearest class from `base` up that defines the method `name` gives
/// it type parameters (GEN-01).
fn generic_method_in_bases(classes: Classes<'_>, base: Option<ClassId>, name: &Identifier) -> bool {
    std::iter::successors(base.and_then(|class| classes.get(class)), |info| {
        info.base.and_then(|class| classes.get(class))
    })
    .find(|info| info.method(&name.name).is_some())
    .is_some_and(|info| info.generic_methods.contains(&name.name))
}

/// Reports a method with type parameters that TyPy 0.5 accepts (GEN-09).
fn planned_generic_method(name: &Identifier, what: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        name.span,
        format!("{what} planned for TyPy 0.5 (GEN-09)"),
    )
    .with_rule(RuleId::GEN_09)
}

/// Returns the method `name` of `base` or of the nearest class it derives from that defines
/// it, with that class's name.
fn inherited_method<'infos>(
    classes: Classes<'infos>,
    base: Option<ClassId>,
    name: &str,
) -> Option<(FunctionId, &'infos str)> {
    let mut current = base;
    while let Some(info) = current.and_then(|class| classes.get(class)) {
        if let Some(method) = info.method(name) {
            return Some((method, &info.name));
        }
        current = info.base;
    }
    None
}

/// Reports a name a class gives two meanings: a class variable that is also an attribute or a
/// method, or an attribute that is also a method.
fn report_name_conflicts(info: &ClassInfo, span: Span, diagnostics: &mut Diagnostics) {
    for name in &info.class_variables {
        if info.attribute(name).is_some() || info.method(name).is_some() {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    span,
                    format!(
                        "`{}.{name}` is a class variable and also an attribute or a method",
                        info.name
                    ),
                )
                .with_rule(RuleId::STD_04),
            );
        }
    }
    for (name, _) in &info.methods {
        if info.attribute(name).is_some() {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    span,
                    format!("`{}.{name}` is both an attribute and a method", info.name),
                )
                .with_rule(RuleId::STD_04),
            );
        }
    }
}

/// Declares what an annotation or an assignment in a class body names: an attribute, with
/// or without a default, or a class variable (CLS-01, CLS-11, ADR-0024).
fn class_body_name(
    statement: &Stmt,
    info: &mut ClassInfo,
    scope: &AnnotationScope<'_>,
    names: &crate::check::ImportedNames,
    diagnostics: &mut Diagnostics,
) {
    match statement {
        Stmt::AnnAssign {
            target: Expr::Name { id, span },
            annotation,
            value,
            ..
        } => {
            // A refused value leaves the attribute declared with its default, so its uses are
            // still checked; the program is not compiled.
            let refused = value.is_some()
                && refuse_shared_parameter(id, annotation, info, scope, diagnostics);
            // A `Final` the class body gives a value is a class variable, as PEP 591 has it.
            let class_var =
                super::annotation::class_var_type(annotation, &names.names, &names.standard)
                    .is_some()
                    || super::annotation::final_type(annotation, &names.names, &names.standard)
                        .is_some();
            if class_var && value.is_some() {
                if !refused {
                    info.class_variables.push(id.clone());
                }
                return;
            }
            let ty = annotation_type(annotation, scope, diagnostics);
            let ty = super::protocol::outside_signature(ty, annotation.span(), diagnostics);
            declare(info, id, Some(ty), *span, diagnostics);
            if value.is_some() {
                default_value(info, id);
            }
        }
        Stmt::Assign { targets, value, .. } => {
            if let [Expr::Name { id, .. }] = targets.as_slice() {
                if id == "__match_args__" {
                    info.match_args = Some(match_arg_names(value));
                }
                info.class_variables.push(id.clone());
            }
        }
        _ => {}
    }
}

/// Reports a value a generic class's body gives whose annotation names a type parameter of
/// the class: the module variable that holds it is shared by every copy, so it cannot take
/// each copy's type (GEN-11). Returns true when it reports one.
fn refuse_shared_parameter(
    id: &str,
    annotation: &Expr,
    info: &ClassInfo,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> bool {
    let Some(parameter) = scope
        .parameters
        .keys()
        .find(|name| super::generic_class::annotation_names(annotation, &[name]))
    else {
        return false;
    };
    diagnostics.push(
        Diagnostic::new(
            DiagnosticCode::error(201),
            annotation.span(),
            format!(
                "a value of the generic class `{}` whose type names its type parameter \
                 `{parameter}` is planned for TyPy 1.0 (GEN-11)",
                info.name
            ),
        )
        .with_rule(RuleId::GEN_11)
        .with_help(format!(
            "declare `{id}` without a value and assign it in `__init__`"
        )),
    );
    true
}

/// Returns the names a `__match_args__` tuple of `str` literals holds, or where a value of
/// another form is.
fn match_arg_names(value: &Expr) -> Result<Vec<String>, Span> {
    let Expr::Tuple { elements, .. } = value else {
        return Err(value.span());
    };
    elements
        .iter()
        .map(|element| match element {
            Expr::Constant {
                value: Constant::Str(name),
                ..
            } => Ok(name.clone()),
            other => Err(other.span()),
        })
        .collect()
}

/// Notes that the class body gives the attribute `name` a default, which an instance, and
/// an exception once the runtime makes it, holds before `__init__` runs (ADR-0024).
fn default_value(info: &mut ClassInfo, name: &str) {
    // A subclass may give an inherited attribute a default of its own, which its instances
    // hold in place of the base's.
    if let Some(index) = info.attribute(name) {
        info.defaults.push(index);
    }
}

/// Declares an attribute in the class body; a second declaration is an error.
fn declare(
    info: &mut ClassInfo,
    name: &str,
    ty: Option<Type>,
    span: Span,
    diagnostics: &mut Diagnostics,
) {
    if let Some(index) = info.attribute(name).filter(|index| *index < info.inherited) {
        // A subclass may declare an attribute of its base again with the same type, so code
        // that knows only the base reads it at the same place with the same type (ADR-0021).
        let base = info.attributes[index].ty.clone();
        if base.as_ref() == ty.as_ref() {
            return;
        }
        let message = match base {
            Some(base) => format!(
                "`{}.{name}` is declared as {}, but its base declares it as {base}",
                info.name,
                ty.map_or_else(|| "something else".to_owned(), |ty| ty.to_string()),
            ),
            None => format!(
                "`{}.{name}` is declared again, but its base assigns it without declaring it",
                info.name
            ),
        };
        let diagnostic =
            Diagnostic::new(DiagnosticCode::error(202), span, message).with_rule(RuleId::CLS_04);
        diagnostics.push(if info.attributes[index].ty.is_none() {
            diagnostic.with_help(format!("declare `{name}` in the body of the base class"))
        } else {
            diagnostic
        });
        return;
    }
    if info.attribute(name).is_some() {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(202),
                span,
                format!("`{}.{name}` is declared more than once", info.name),
            )
            .with_rule(RuleId::STD_04),
        );
        return;
    }
    info.attributes.push(AttributeInfo {
        name: name.to_owned(),
        ty,
    });
}

/// Checks the signature of a special method the runtime or a statement calls: `__str__`,
/// `__repr__`, `__exit__`, `__iter__`, `__next__` and `__enter__`.
fn check_special_method(method: &Definition<'_>, name: &Identifier, diagnostics: &mut Diagnostics) {
    match name.name.as_str() {
        "__str__" | "__repr__" => check_text_method(method, name, diagnostics),
        "__exit__" => check_exit(method, name, diagnostics),
        "__eq__" | "__ne__" | "__lt__" | "__le__" | "__gt__" | "__ge__" => {
            check_comparison(method, name, diagnostics);
        }
        "__len__" | "__bool__" | "__getitem__" | "__setitem__" | "__delitem__" | "__contains__" => {
            check_container_method(method, name, diagnostics);
        }
        "__neg__" | "__pos__" | "__invert__" | "__abs__"
            if method.signature.parameters.len() != 1 =>
        {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    name.span,
                    format!("`{}` takes only `self`", name.name),
                )
                .with_rule(RuleId::CLS_07),
            );
        }
        arithmetic if is_binary_method(arithmetic) && method.signature.parameters.len() != 2 => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    name.span,
                    format!("`{}` takes `self` and the other operand", name.name),
                )
                .with_rule(RuleId::CLS_07),
            );
        }
        "__hash__"
            if method.signature.parameters.len() != 1
                || !method.signature.returns.matches(&Type::Int) =>
        {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    name.span,
                    "`__hash__` takes only `self` and returns an int",
                )
                .with_rule(RuleId::CLS_07),
            );
        }
        "__iter__" | "__next__" | "__enter__" if method.signature.parameters.len() != 1 => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    name.span,
                    format!("`{}` takes only `self`", name.name),
                )
                .with_rule(RuleId::CST_08),
            );
        }
        _ => {}
    }
}

/// Returns true for the methods of the binary arithmetic and bitwise operators and their
/// reflected forms, such as `__add__` and `__radd__` (CLS-07).
pub(super) fn is_binary_method(name: &str) -> bool {
    let Some(bare) = name
        .strip_prefix("__")
        .and_then(|rest| rest.strip_suffix("__"))
    else {
        return false;
    };
    // A reflected method and an in-place one take the operands a binary one does (CLS-20).
    let bare = bare
        .strip_prefix('r')
        .or_else(|| bare.strip_prefix('i'))
        .filter(|rest| BINARY.contains(rest))
        .unwrap_or(bare);
    BINARY.contains(&bare)
}

/// The binary operators' method names without their underscores.
const BINARY: [&str; 13] = [
    "add", "sub", "mul", "matmul", "truediv", "floordiv", "mod", "pow", "lshift", "rshift", "and",
    "or", "xor",
];

/// Checks a container method's signature: `__len__` returns an `int` and `__bool__` and
/// `__contains__` a `bool`, `__setitem__` and `__delitem__` return `None`, and each takes
/// `self` and the values its operation passes (CLS-07).
fn check_container_method(
    method: &Definition<'_>,
    name: &Identifier,
    diagnostics: &mut Diagnostics,
) {
    let signature = &method.signature;
    let (parameters, returns, written) = match name.name.as_str() {
        "__len__" => (1, Some(Type::Int), "takes only `self` and returns an int"),
        "__bool__" => (1, Some(Type::Bool), "takes only `self` and returns a bool"),
        "__contains__" => (
            2,
            Some(Type::Bool),
            "takes `self` and an item, and returns a bool",
        ),
        "__getitem__" => (2, None, "takes `self` and a key"),
        "__setitem__" => (
            3,
            Some(Type::NoneType),
            "takes `self`, a key and a value, and returns `None`",
        ),
        _ => (
            2,
            Some(Type::NoneType),
            "takes `self` and a key, and returns `None`",
        ),
    };
    let fits = signature.parameters.len() == parameters
        && returns.is_none_or(|returns| signature.returns.matches(&returns));
    if !fits {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                name.span,
                format!("`{}` {written}", name.name),
            )
            .with_rule(RuleId::CLS_07),
        );
    }
}

/// A comparison method takes `self` and one more value, of a type that accepts every instance
/// of its class, so a comparison inside a container always calls it with one, and returns a
/// `bool` (CLS-07, ADR-0026).
fn check_comparison(method: &Definition<'_>, name: &Identifier, diagnostics: &mut Diagnostics) {
    let signature = &method.signature;
    let fits = signature.parameters.len() == 2
        && signature.parameters[0]
            .ty
            .is_assignable_to(&signature.parameters[1].ty)
        && signature.returns.implemented().matches(&Type::Bool);
    if !fits {
        let class = signature
            .parameters
            .first()
            .map(|parameter| parameter.ty.clone());
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                name.span,
                format!(
                    "`{}` takes `self` and a value its class's instances are, and returns a bool",
                    name.name
                ),
            )
            .with_rule(RuleId::CLS_07)
            .with_help(class.map_or_else(String::new, |class| {
                format!(
                    "write `def {}(self, other: \"{class}\") -> bool:`",
                    name.name
                )
            })),
        );
    }
}

/// `__str__` and `__repr__` take only the instance and return a `str`, as CPython requires
/// (CLS-08).
fn check_text_method(method: &Definition<'_>, name: &Identifier, diagnostics: &mut Diagnostics) {
    let signature = &method.signature;
    if signature.parameters.len() != 1 || !signature.returns.matches(&Type::Str) {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                name.span,
                format!("`{}` takes only `self` and returns a str", name.name),
            )
            .with_rule(RuleId::CLS_08),
        );
    }
}

/// `__exit__` takes `self` and the three parameters of CST-10, which accept the types CPython
/// passes, and returns `None`, a `bool` or `bool | None`, a true value suppressing the
/// exception (CST-05).
fn check_exit(method: &Definition<'_>, name: &Identifier, diagnostics: &mut Diagnostics) {
    let signature = &method.signature;
    let parameters_fit = signature.parameters.len() == 4
        && signature.parameters[1..]
            .iter()
            .enumerate()
            .all(|(index, parameter)| {
                super::definition::exit_parameter(index + 1).is_assignable_to(&parameter.ty)
            });
    let returns_fit = matches!(signature.returns, Type::NoneType | Type::Bool | Type::Error)
        || signature.returns == Type::optional(Type::Bool);
    if !parameters_fit || !returns_fit {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                name.span,
                "`__exit__` takes `self`, the class of the exception, the exception and its \
                 traceback, and returns `None` or a `bool`",
            )
            .with_rule(RuleId::CST_05),
        );
    }
}

/// Reports a special method a class may not define yet.
fn planned_method(name: &Identifier) -> Diagnostic {
    // The special methods TyPy does not take, such as `__call__`, come after 1.0.
    Diagnostic::new(
        DiagnosticCode::error(201),
        name.span,
        format!(
            "the special method `{}` is planned for a release after TyPy 1.0 (CLS-07)",
            name.name
        ),
    )
    .with_rule(RuleId::CLS_07)
}

/// Returns the attributes `__init__` assigns on its first parameter, in source order, at any
/// depth of its blocks; an augmented assignment reads the attribute first, so it assigns none.
fn initialized_attributes(parts: &FunctionParts<'_>) -> Vec<String> {
    fn assigned(target: &Expr, instance: &str, found: &mut Vec<String>) {
        match target {
            Expr::Attribute {
                value, attribute, ..
            } if matches!(value.as_ref(), Expr::Name { id, .. } if id == instance) => {
                if !found.contains(&attribute.name) {
                    found.push(attribute.name.clone());
                }
            }
            Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
                for element in elements {
                    assigned(element, instance, found);
                }
            }
            _ => {}
        }
    }
    fn statements(body: &[Stmt], instance: &str, found: &mut Vec<String>) {
        for statement in body {
            match statement {
                Stmt::Assign { targets, .. } => {
                    for each in targets {
                        assigned(each, instance, found);
                    }
                }
                // An annotation without a value assigns nothing.
                Stmt::AnnAssign {
                    target: each,
                    value: Some(_),
                    ..
                } => assigned(each, instance, found),
                Stmt::For {
                    target: each,
                    body,
                    orelse,
                    ..
                } => {
                    assigned(each, instance, found);
                    statements(body, instance, found);
                    statements(orelse, instance, found);
                }
                Stmt::While { body, orelse, .. } | Stmt::If { body, orelse, .. } => {
                    statements(body, instance, found);
                    statements(orelse, instance, found);
                }
                Stmt::Match { cases, .. } => {
                    for case in cases {
                        statements(&case.body, instance, found);
                    }
                }
                Stmt::Try { .. } => {
                    for block in statement.try_blocks() {
                        statements(block, instance, found);
                    }
                }
                Stmt::With { body, .. } => statements(body, instance, found),
                _ => {}
            }
        }
    }
    let mut found = Vec::new();
    if let Some(instance) = parts.parameters.first() {
        statements(parts.body, &instance.name.name, &mut found);
    }
    found
}

/// A value a class body gives a name: a class variable, or the default of an instance
/// attribute, with the assignment that gives it, to the module variable `Class.name` that
/// holds it, which runs where the `class` statement runs (CLS-11, ADR-0024).
pub(crate) struct ClassValue {
    pub(crate) name: String,
    pub(crate) assignment: Stmt,
}

/// Returns the name of the module variable that holds a class's value `name`.
pub(crate) fn class_value_variable(class: &str, name: &str) -> String {
    format!("{class}.{name}")
}

/// Returns the values a class body gives, in source order: `name = value`, and
/// `name: ClassVar[T] = value`, for a class variable, and `name: T = value` for the default
/// of an instance attribute.
pub(crate) fn class_values(
    (class, parameters): (&str, &[typy_source::syntax::TypeParameter]),
    body: &[Stmt],
    names: &BTreeMap<String, super::annotation::AnnotationName>,
    standard: &BTreeMap<String, String>,
) -> Vec<ClassValue> {
    let renamed = |id: &str, span: Span| Expr::Name {
        id: class_value_variable(class, id),
        span,
    };
    let parameters: Vec<&str> = parameters
        .iter()
        .map(|parameter| parameter.name.name.as_str())
        .collect();
    body.iter()
        .filter_map(|statement| match statement {
            // A value whose type names a type parameter of the class is refused (GEN-11).
            Stmt::AnnAssign { annotation, .. }
                if super::generic_class::annotation_names(annotation, &parameters) =>
            {
                None
            }
            Stmt::AnnAssign {
                target: Expr::Name { id, span },
                annotation,
                value: Some(value),
                simple,
                span: whole,
            } => {
                let target = renamed(id, *span);
                let assignment =
                    match super::annotation::class_var_type(annotation, names, standard) {
                        Some(super::annotation::ClassVarAnnotation::Typed(inner)) => {
                            Stmt::AnnAssign {
                                target,
                                annotation: inner.clone(),
                                value: Some(value.clone()),
                                simple: *simple,
                                span: *whole,
                            }
                        }
                        Some(super::annotation::ClassVarAnnotation::Bare) => Stmt::Assign {
                            targets: vec![target],
                            value: value.clone(),
                            span: *whole,
                        },
                        None => Stmt::AnnAssign {
                            target,
                            annotation: annotation.clone(),
                            value: Some(value.clone()),
                            simple: *simple,
                            span: *whole,
                        },
                    };
                Some(ClassValue {
                    name: id.clone(),
                    assignment,
                })
            }
            Stmt::Assign {
                targets,
                value,
                span,
            } => match targets.as_slice() {
                [Expr::Name { id, span: at }] => Some(ClassValue {
                    name: id.clone(),
                    assignment: Stmt::Assign {
                        targets: vec![renamed(id, *at)],
                        value: value.clone(),
                        span: *span,
                    },
                }),
                _ => None,
            },
            _ => None,
        })
        .collect()
}
