//! `@dataclass` (CLS-05): a class with the decorator gets the methods CPython's `dataclasses`
//! module writes for it, `__init__`, `__repr__`, `__eq__` and, with `order=True`, the ordering
//! methods, as source the checker reads as if the program had written it, at the decorator.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use camino::Utf8PathBuf;
use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, FileId, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Keyword, Stmt};
use typy_source::{Module, PythonFloor, SourceFile, parse};

/// A field of a dataclass: its name, annotation and default, in the order the class body
/// declares it, after its dataclass bases' fields.
#[derive(Clone, Debug)]
pub(crate) struct Field {
    name: String,
    annotation: Expr,
    default: Option<Expr>,
}

/// What the expansion of a dataclass tells the checker: whether its instances are frozen, and
/// whether it wrote the `__init__` that alone may assign a frozen instance's fields.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DataclassInfo {
    pub(crate) frozen: bool,
    pub(crate) generated_init: bool,
}

/// The dataclasses of the modules expanded so far: each one's fields, by module and class.
#[derive(Debug, Default)]
pub(crate) struct Dataclasses {
    fields: BTreeMap<(String, String), Vec<Field>>,
    /// The class each dataclass's comparison methods take: the first dataclass of its chain of
    /// bases, so a subclass's methods take what its base's take (ADR-0021).
    compared: BTreeMap<(String, String), String>,
    /// Whether each dataclass of the module being expanded is frozen, before its module's
    /// information is complete.
    pending: BTreeMap<(String, String), bool>,
    /// For each module, its dataclasses by name.
    pub(crate) info: BTreeMap<String, BTreeMap<String, DataclassInfo>>,
}

/// The names a module binds to the `dataclasses` module and its `dataclass` and `field`.
#[derive(Default)]
struct Names {
    modules: BTreeSet<String>,
    dataclass: BTreeSet<String>,
    field: BTreeSet<String>,
    /// Each name a `from module import Class` binds, with the module and the class.
    imported: BTreeMap<String, (String, String)>,
}

/// The options of `@dataclass(...)` CLS-05 covers.
#[derive(Clone, Copy)]
struct Options {
    eq: bool,
    order: bool,
    frozen: bool,
}

impl Dataclasses {
    /// Returns the module with each dataclass given its methods, and the problems found.
    pub(crate) fn expand(&mut self, module: &Module, diagnostics: &mut Diagnostics) -> Module {
        let names = Names::of(&module.syntax.body);
        if names.dataclass.is_empty() && names.modules.is_empty() {
            return module.clone();
        }
        let mut expanded = module.clone();
        let module_name = module.name.as_str().to_owned();
        let mut info = BTreeMap::new();
        for statement in &mut expanded.syntax.body {
            if let Some((class, class_info)) =
                self.expand_class(statement, &module_name, &names, diagnostics)
            {
                info.insert(class, class_info);
            }
        }
        self.info.insert(module_name, info);
        expanded
    }

    /// Gives one class with `@dataclass` its methods, and returns its name and what the
    /// checker needs of it; `None` for any other statement and after a reported problem.
    fn expand_class(
        &mut self,
        statement: &mut Stmt,
        module_name: &str,
        names: &Names,
        diagnostics: &mut Diagnostics,
    ) -> Option<(String, DataclassInfo)> {
        let Stmt::ClassDef {
            name,
            bases,
            body,
            decorators,
            ..
        } = statement
        else {
            return None;
        };
        let position = decorators
            .iter()
            .position(|decorator| names.is_dataclass(decorator))?;
        let decorator = decorators.remove(position);
        let options = options(&decorator, diagnostics)?;
        let at = decorator.span();
        let inherited = bases
            .first()
            .and_then(|base| self.base_fields(base, module_name, names));
        let compared = match (bases.first(), &inherited) {
            (Some(base), Some(_)) => self.compared_class(base, module_name, names),
            _ => name.name.clone(),
        };
        if inherited.is_some()
            && let Some(base_frozen) = bases
                .first()
                .and_then(|base| self.base_frozen(base, module_name, names))
            && base_frozen != options.frozen
        {
            let (this, that) = if options.frozen {
                ("frozen", "non-frozen")
            } else {
                ("non-frozen", "frozen")
            };
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    at,
                    format!(
                        "a {this} dataclass cannot derive from a {that} one, as CPython \
                         refuses"
                    ),
                )
                .with_rule(RuleId::CLS_05),
            );
            return None;
        }
        let inherited = inherited.unwrap_or_default();
        let reported = diagnostics.iter().count();
        let own = own_fields(body, names, diagnostics);
        // A refused field would give the written methods errors of their own.
        if diagnostics.iter().count() > reported {
            return None;
        }
        let mut fields = inherited;
        for field in own {
            match fields.iter_mut().find(|each| each.name == field.name) {
                Some(each) => *each = field,
                None => fields.push(field),
            }
        }
        if !fields_are_ordered(&fields, at, diagnostics) {
            return None;
        }
        let defined: BTreeSet<String> = body
            .iter()
            .filter_map(|statement| match statement {
                Stmt::FunctionDef { name, .. } => Some(name.name.clone()),
                _ => None,
            })
            .collect();
        let generated = methods(
            [&name.name, &compared],
            &fields,
            options,
            &defined,
            at,
            diagnostics,
        );
        let generated_init = generated.iter().any(
            |method| matches!(method, Stmt::FunctionDef { name, .. } if name.name == "__init__"),
        );
        body.extend(generated);
        let class_info = DataclassInfo {
            frozen: options.frozen,
            generated_init,
        };
        self.fields
            .insert((module_name.to_owned(), name.name.clone()), fields);
        self.compared
            .insert((module_name.to_owned(), name.name.clone()), compared);
        self.pending
            .insert((module_name.to_owned(), name.name.clone()), options.frozen);
        Some((name.name.clone(), class_info))
    }

    /// Returns whether a dataclass base is frozen.
    fn base_frozen(&self, base: &Expr, module: &str, names: &Names) -> Option<bool> {
        let Expr::Name { id, .. } = base else {
            return None;
        };
        let (module, class) = names
            .imported
            .get(id)
            .cloned()
            .unwrap_or_else(|| (module.to_owned(), id.clone()));
        let key = (module, class);
        self.pending.get(&key).copied().or_else(|| {
            self.info
                .get(&key.0)
                .and_then(|classes| classes.get(&key.1))
                .map(|info| info.frozen)
        })
    }

    /// Returns the class a dataclass base's comparison methods take, by a name this module
    /// binds: the base's own for a base of another module.
    fn compared_class(&self, base: &Expr, module: &str, names: &Names) -> String {
        let Expr::Name { id, .. } = base else {
            return String::new();
        };
        if names.imported.contains_key(id) {
            return id.clone();
        }
        self.compared
            .get(&(module.to_owned(), id.clone()))
            .cloned()
            .unwrap_or_else(|| id.clone())
    }

    /// Returns the fields of a base that is a dataclass of this module or one a
    /// `from module import Class` names.
    fn base_fields(&self, base: &Expr, module: &str, names: &Names) -> Option<Vec<Field>> {
        let Expr::Name { id, .. } = base else {
            return None;
        };
        let key = names
            .imported
            .get(id)
            .cloned()
            .unwrap_or_else(|| (module.to_owned(), id.clone()));
        self.fields.get(&key).cloned()
    }
}

impl Names {
    /// Returns the names the module's imports bind to `dataclasses`, `dataclass` and `field`.
    fn of(statements: &[Stmt]) -> Self {
        let mut names = Self::default();
        for statement in statements {
            match statement {
                Stmt::Import { names: aliases, .. } => {
                    for alias in aliases
                        .iter()
                        .filter(|alias| alias.name.name == "dataclasses")
                    {
                        let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
                        names.modules.insert(bound.name.clone());
                    }
                }
                Stmt::ImportFrom {
                    module: Some(module),
                    names: aliases,
                    level: 0,
                    ..
                } => {
                    for alias in aliases {
                        let bound = alias.as_name.as_ref().unwrap_or(&alias.name).name.clone();
                        match (module.name.as_str(), alias.name.name.as_str()) {
                            ("dataclasses", "dataclass") => {
                                names.dataclass.insert(bound);
                            }
                            ("dataclasses", "field") => {
                                names.field.insert(bound);
                            }
                            _ => {
                                names
                                    .imported
                                    .insert(bound, (module.name.clone(), alias.name.name.clone()));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        names
    }

    /// Returns true when an expression names `name` of `dataclasses`: bound by `from` or read
    /// through the module.
    fn names(&self, expression: &Expr, bound: &BTreeSet<String>, name: &str) -> bool {
        match expression {
            Expr::Name { id, .. } => bound.contains(id),
            Expr::Attribute {
                value, attribute, ..
            } => {
                attribute.name == name
                    && matches!(value.as_ref(), Expr::Name { id, .. } if self.modules.contains(id))
            }
            _ => false,
        }
    }

    /// Returns true for `@dataclass` and `@dataclass(...)`.
    fn is_dataclass(&self, decorator: &Expr) -> bool {
        let called = match decorator {
            Expr::Call { function, .. } => function.as_ref(),
            other => other,
        };
        self.names(called, &self.dataclass, "dataclass")
    }

    /// Returns the arguments of `field(...)`, or `None` for any other expression.
    fn field_call<'call>(&self, expression: &'call Expr) -> Option<FieldCall<'call>> {
        match expression {
            Expr::Call {
                function,
                arguments,
                keywords,
                span,
            } if self.names(function, &self.field, "field") => Some(FieldCall {
                arguments,
                keywords,
                span: *span,
            }),
            _ => None,
        }
    }
}

/// Returns the options of a `@dataclass` decorator, or `None` after reporting one CLS-05 does
/// not cover.
fn options(decorator: &Expr, diagnostics: &mut Diagnostics) -> Option<Options> {
    let mut options = Options {
        eq: true,
        order: false,
        frozen: false,
    };
    let Expr::Call {
        arguments,
        keywords,
        ..
    } = decorator
    else {
        return Some(options);
    };
    if let Some(argument) = arguments.first() {
        diagnostics.push(planned(
            argument.span(),
            "`@dataclass` with positional arguments",
        ));
        return None;
    }
    for Keyword { name, value, span } in keywords {
        let flag = match value {
            Expr::Constant {
                value: Constant::Bool(flag),
                ..
            } => Some(*flag),
            _ => None,
        };
        match (name.as_ref().map(|name| name.name.as_str()), flag) {
            (Some("eq"), Some(flag)) => options.eq = flag,
            (Some("order"), Some(flag)) => options.order = flag,
            (Some("frozen"), Some(flag)) => options.frozen = flag,
            (Some(option), _) => {
                diagnostics.push(planned(
                    *span,
                    &format!("the `@dataclass` option `{option}`, or one given other than as `True` or `False`,"),
                ));
                return None;
            }
            (None, _) => {
                diagnostics.push(planned(*span, "`@dataclass(**options)`"));
                return None;
            }
        }
    }
    if options.order && !options.eq {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                decorator.span(),
                "`order=True` needs `eq=True`, as CPython requires",
            )
            .with_rule(RuleId::CLS_05),
        );
        return None;
    }
    Some(options)
}

/// Returns the fields the class body declares, and gives each `name: T = field(default=v)` the
/// value `v`, as CPython's class attribute is.
fn own_fields(body: &mut [Stmt], names: &Names, diagnostics: &mut Diagnostics) -> Vec<Field> {
    let mut fields = Vec::new();
    for statement in body {
        let Stmt::AnnAssign {
            target: Expr::Name { id, .. },
            annotation,
            value,
            ..
        } = statement
        else {
            continue;
        };
        if is_class_var(annotation) {
            continue;
        }
        if let Some(call) = value.as_ref().and_then(|value| names.field_call(value)) {
            *value = field_default(&call, diagnostics);
        }
        if let Some(default) = value.as_ref()
            && let Some(kind) = mutable(default)
        {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    default.span(),
                    format!(
                        "a mutable default <class '{kind}'> for the field `{id}` is not allowed, \
                         as CPython raises `ValueError` for it"
                    ),
                )
                .with_rule(RuleId::CLS_05),
            );
        }
        fields.push(Field {
            name: id.clone(),
            annotation: annotation.clone(),
            default: value.clone(),
        });
    }
    fields
}

/// The arguments of a call of `field`.
struct FieldCall<'call> {
    arguments: &'call [Expr],
    keywords: &'call [Keyword],
    span: Span,
}

/// Returns the default `field(default=v)` gives, `v`, or none for `field()`.
fn field_default(call: &FieldCall<'_>, diagnostics: &mut Diagnostics) -> Option<Expr> {
    if !call.arguments.is_empty() {
        diagnostics.push(planned(call.span, "`field` with positional arguments"));
        return None;
    }
    let mut default = None;
    for keyword in call.keywords {
        match keyword.name.as_ref().map(|name| name.name.as_str()) {
            Some("default") => default = Some(keyword.value.clone()),
            Some(option) => {
                diagnostics.push(planned(keyword.span, &format!("`field({option}=...)`")));
            }
            None => diagnostics.push(planned(keyword.span, "`field(**options)`")),
        }
    }
    default
}

/// Returns the class of a display that makes a mutable value: `list`, `dict` or `set`.
fn mutable(default: &Expr) -> Option<&'static str> {
    match default {
        Expr::List { .. } | Expr::ListComp { .. } => Some("list"),
        Expr::Dict { .. } | Expr::DictComp { .. } => Some("dict"),
        Expr::Set { .. } | Expr::SetComp { .. } => Some("set"),
        _ => None,
    }
}

/// Returns true for `ClassVar` and `ClassVar[T]`, by name or through `typing`.
fn is_class_var(annotation: &Expr) -> bool {
    let named = match annotation {
        Expr::Subscript { value, .. } => value.as_ref(),
        other => other,
    };
    match named {
        Expr::Name { id, .. } => id == "ClassVar",
        Expr::Attribute { attribute, .. } => attribute.name == "ClassVar",
        _ => false,
    }
}

/// Reports a field without a default after one with a default, which CPython refuses since
/// `__init__` could not take them in that order.
fn fields_are_ordered(fields: &[Field], at: Span, diagnostics: &mut Diagnostics) -> bool {
    let mut defaulted = None;
    for field in fields {
        match (&field.default, defaulted) {
            (Some(_), _) => defaulted = Some(&field.name),
            (None, Some(before)) => {
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        field.annotation.span(),
                        format!(
                            "the field `{}` has no default and follows `{before}`, which has one, \
                             as CPython refuses",
                            field.name
                        ),
                    )
                    .with_rule(RuleId::CLS_05)
                    .with_label(at, "the dataclass"),
                );
                return false;
            }
            (None, None) => {}
        }
    }
    true
}

/// Returns the methods CPython's `dataclasses` writes for the class, those the class does not
/// define itself, at the decorator.
fn methods(
    [class, compared]: [&str; 2],
    fields: &[Field],
    options: Options,
    defined: &BTreeSet<String>,
    at: Span,
    diagnostics: &mut Diagnostics,
) -> Vec<Stmt> {
    let mut source = String::new();
    let own = |name: &str| defined.contains(name);
    if !own("__init__") {
        write_init(&mut source, fields);
    }
    if !own("__repr__") {
        write_repr(&mut source, fields);
    }
    let (mine, theirs) = (tuple(fields, "self"), tuple(fields, "other"));
    // The methods take what a base's take, and compare only an object of the class itself,
    // as CPython does; the `isinstance` test, true then, lets them read its fields.
    if options.eq && !own("__eq__") {
        let _ = write!(
            source,
            "def __eq__(self, other: \"{compared}\") -> bool:\n    if type(other) is not type(self) or not isinstance(other, {class}):\n        return False\n    return {mine} == {theirs}\n"
        );
    }
    let orders = [
        ("__lt__", "<"),
        ("__le__", "<="),
        ("__gt__", ">"),
        ("__ge__", ">="),
    ];
    for (method, operator) in orders.into_iter().filter(|_| options.order) {
        if own(method) {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    at,
                    format!(
                        "a dataclass with `order=True` cannot define `{method}`, as CPython refuses"
                    ),
                )
                .with_rule(RuleId::CLS_05),
            );
            continue;
        }
        let _ = write!(
            source,
            "def {method}(self, other: \"{compared}\") -> bool:\n    if not isinstance(other, {class}):\n        return False\n    return {mine} {operator} {theirs}\n"
        );
    }
    parsed(&source, fields, at)
}

/// Writes `__init__`, which takes the fields in order and assigns each; the annotations and
/// defaults written stand for the fields' own, which `parsed` puts in their place.
fn write_init(source: &mut String, fields: &[Field]) {
    let _ = write!(source, "def __init__(self");
    for field in fields {
        let default = if field.default.is_some() { " = 0" } else { "" };
        let _ = write!(source, ", {}: int{default}", field.name);
    }
    let _ = writeln!(source, ") -> None:");
    if fields.is_empty() {
        let _ = writeln!(source, "    pass");
    }
    for field in fields {
        let _ = writeln!(source, "    self.{0} = {0}", field.name);
    }
}

/// Writes `__repr__`, `Class(name=value, ...)` with each value's `repr`, named by the class of
/// the instance, as CPython names it.
fn write_repr(source: &mut String, fields: &[Field]) {
    let _ = write!(
        source,
        "def __repr__(self) -> str:\n    return type(self).__name__ + \"(\""
    );
    for (position, field) in fields.iter().enumerate() {
        let separator = if position == 0 { "" } else { ", " };
        let _ = write!(
            source,
            " + \"{separator}{0}=\" + repr(self.{0})",
            field.name
        );
    }
    let _ = writeln!(source, " + \")\"");
}

/// Returns the methods `source` holds, each moved to the decorator, `__init__` with the
/// fields' own annotations and defaults.
fn parsed(source: &str, fields: &[Field], at: Span) -> Vec<Stmt> {
    let file = SourceFile::from_text(FileId(u32::MAX), Utf8PathBuf::from("<dataclass>"), source);
    // The source is written above and always parses.
    let mut methods = parse(&file, PythonFloor::default())
        .map(|tree| tree.body)
        .unwrap_or_default();
    for method in &mut methods {
        method.relocate(at);
        // Each statement written is a `def`.
        match method {
            Stmt::FunctionDef {
                name, parameters, ..
            } if name.name == "__init__" => {
                for (parameter, field) in parameters.iter_mut().skip(1).zip(fields) {
                    parameter.annotation = Some(field.annotation.clone());
                    parameter.default.clone_from(&field.default);
                }
            }
            _ => {}
        }
    }
    methods
}

/// Returns `(name.a, name.b, )`, the tuple of the fields of `name`.
fn tuple(fields: &[Field], name: &str) -> String {
    let mut items = String::from("(");
    for field in fields {
        let _ = write!(items, "{name}.{}, ", field.name);
    }
    items.push(')');
    items
}

/// Returns the error for a form of `dataclasses` CLS-05 does not cover yet.
fn planned(span: Span, what: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        format!("{what} is planned for TyPy 0.5 (CLS-05)"),
    )
    .with_rule(RuleId::CLS_05)
}
