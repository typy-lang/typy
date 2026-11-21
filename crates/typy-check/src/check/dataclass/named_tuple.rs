//! `class Point(NamedTuple)` (CLS-27, ADR-0088): a class whose base names `typing.NamedTuple`
//! gets the methods CPython's `typing` gives it, as source the checker reads as if the program
//! had written it: `__init__` from the fields and their defaults, `__repr__`, `__eq__`,
//! `__hash__` and, for fields of types that order, the order methods, each over the tuple of
//! the fields; the hidden method that returns that tuple, which the checker calls where a
//! tuple operation applies; `_asdict`, `__match_args__` and `_fields`; and the hidden method
//! `_replace` calls, which a class derived from a named tuple gets too, so it makes an instance
//! of the receiver's own class. What CPython raises when the class statement runs is reported
//! with its text, and a class derived from a named tuple is checked for what it may add.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{BinaryOperator, Constant, Expr, Stmt};

use super::{
    DataclassInfo, Dataclasses, Field, Names, defined, match_args_statement, parsed, tuple,
    write_init, write_repr,
};
use crate::class_type::{NAMED_TUPLE_FIELDS, NAMED_TUPLE_REPLACE};

/// The names CPython's `NamedTuple` keeps for itself, which a class body may not define.
const PROHIBITED: [&str; 10] = [
    "__new__",
    "__init__",
    "__slots__",
    "__getnewargs__",
    "_fields",
    "_field_defaults",
    "_make",
    "_replace",
    "_asdict",
    "_source",
];

/// The methods a named tuple has from `tuple`, which its class or a class derived from it
/// defines from TyPy 1.0 (CLS-41).
const TUPLE_PROTOCOL: [&str; 4] = ["__len__", "__getitem__", "__iter__", "__contains__"];

/// The order methods, each with its operator.
const ORDERS: [(&str, &str); 4] = [
    ("__lt__", "<"),
    ("__le__", "<="),
    ("__gt__", ">"),
    ("__ge__", ">="),
];

impl Dataclasses {
    /// Gives a class derived from `NamedTuple` its methods, or checks a class derived from a
    /// named tuple; returns its name and what the checker needs of it, and `None` for any
    /// other statement and after a reported problem.
    pub(super) fn expand_named_tuple(
        &mut self,
        statement: &mut Stmt,
        module: &str,
        names: &Names,
        diagnostics: &mut Diagnostics,
    ) -> Option<(String, DataclassInfo)> {
        let Stmt::ClassDef {
            name, bases, body, ..
        } = statement
        else {
            return None;
        };
        let class = name.name.clone();
        if let Some(fields) = bases
            .first()
            .and_then(|base| names.class_key(base, module))
            .and_then(|key| self.named_tuples.get(&key))
            .cloned()
        {
            derived_body(body, diagnostics)?;
            body.extend(replace_method(&class, &fields, name.span));
            self.named_tuples
                .insert((module.to_owned(), class.clone()), fields);
            let info = DataclassInfo {
                named_tuple: true,
                ..DataclassInfo::default()
            };
            return Some((class, info));
        }
        let position = bases.iter().position(|base| {
            names.class_key(base, module) == Some(("typing".to_owned(), "NamedTuple".to_owned()))
        })?;
        let at = bases[position].span();
        if let Some(other) = bases
            .iter()
            .find(|base| !is_named_tuple_or_generic(base, names))
        {
            diagnostics.push(class_error(
                other.span(),
                (
                    "TypeError",
                    "can only inherit from a NamedTuple type and Generic",
                ),
            ));
            // The other bases are reported here, so the class check does not report them
            // again.
            bases.retain(|base| is_named_tuple_or_generic(base, names));
            return None;
        }
        let fields = fields(body, diagnostics)?;
        let defined = defined(body);
        if let Some(own) = defined
            .iter()
            .find(|own| PROHIBITED.contains(&own.as_str()))
        {
            diagnostics.push(class_error(
                at,
                (
                    "AttributeError",
                    &format!("Cannot overwrite NamedTuple attribute {own}"),
                ),
            ));
            return None;
        }
        refuse_tuple_protocol(body, "a named tuple that defines", diagnostics)?;
        bases.remove(position);
        let hashable = fields
            .iter()
            .all(|field| !self.unhashable_annotation(&field.annotation, module, names));
        body.extend(methods(&class, &fields, hashable, &defined, at));
        if !defined.contains("__match_args__") {
            body.extend(match_args_statement(&fields, at));
        }
        body.extend(fields_statement(&fields, at));
        body.extend(replace_method(&class, &fields, at));
        self.named_tuples
            .insert((module.to_owned(), class.clone()), fields.clone());
        let info = DataclassInfo {
            frozen: true,
            generated_init: true,
            match_args: (!defined.contains("__match_args__"))
                .then(|| fields.iter().map(|field| field.name.clone()).collect()),
            named_tuple: true,
        };
        Some((class, info))
    }
}

/// Returns true for a base CPython takes beside `NamedTuple`: `NamedTuple` itself and
/// `Generic`, which the class check refuses as it refuses any `Generic` base (GEN-01).
fn is_named_tuple_or_generic(base: &Expr, names: &Names) -> bool {
    let class = match base {
        Expr::Subscript { value, .. } => value.as_ref(),
        other => other,
    };
    let key = names.class_key(class, "");
    matches!(key, Some((module, name)) if module == "typing"
        && matches!(name.as_str(), "NamedTuple" | "Generic"))
}

/// Returns the fields the class body declares, each annotated name in order with its default,
/// or `None` after reporting what CPython raises for them.
fn fields(body: &[Stmt], diagnostics: &mut Diagnostics) -> Option<Vec<Field>> {
    let mut fields: Vec<Field> = Vec::new();
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
        if super::is_class_var(annotation) {
            diagnostics.push(class_error(
                annotation.span(),
                (
                    "TypeError",
                    &format!("the field `{id}` cannot be a `ClassVar`"),
                ),
            ));
            return None;
        }
        fields.push(Field {
            name: id.clone(),
            annotation: annotation.clone(),
            default: value.clone(),
        });
    }
    let mut defaulted: Option<&Field> = None;
    for field in &fields {
        match (&field.default, defaulted) {
            (Some(_), _) => defaulted = Some(field),
            (None, Some(before)) => {
                diagnostics.push(class_error(
                    field.annotation.span(),
                    (
                        "TypeError",
                        &format!(
                            "Non-default namedtuple field {} cannot follow default field {}",
                            field.name, before.name
                        ),
                    ),
                ));
                return None;
            }
            (None, None) => {}
        }
    }
    if let Some(field) = fields.iter().find(|field| field.name.starts_with('_')) {
        diagnostics.push(class_error(
            field.annotation.span(),
            (
                "ValueError",
                &format!(
                    "Field names cannot start with an underscore: '{}'",
                    field.name
                ),
            ),
        ));
        return None;
    }
    Some(fields)
}

/// Checks the body of a class derived from a named tuple: `__new__`, `__init__`, attributes of
/// its own and the methods of the tuple protocol are planned for TyPy 1.0 (CLS-41).
fn derived_body(body: &[Stmt], diagnostics: &mut Diagnostics) -> Option<()> {
    let own_attribute = body.iter().find_map(|statement| match statement {
        Stmt::AnnAssign { annotation, .. } => Some(annotation.span()),
        _ => None,
    });
    if let Some(span) = own_attribute {
        diagnostics.push(planned(
            span,
            "a class derived from a named tuple with an attribute of its own",
        ));
        return None;
    }
    let constructor = body.iter().find_map(|statement| match statement {
        Stmt::FunctionDef { name, .. } if matches!(name.name.as_str(), "__new__" | "__init__") => {
            Some(name)
        }
        _ => None,
    });
    if let Some(name) = constructor {
        diagnostics.push(planned(
            name.span,
            &format!(
                "a class derived from a named tuple that defines `{}`",
                name.name
            ),
        ));
        return None;
    }
    refuse_tuple_protocol(
        body,
        "a class derived from a named tuple that defines",
        diagnostics,
    )
}

/// Reports a method of the tuple protocol the class defines, planned for TyPy 1.0 (CLS-41);
/// `what` names the class.
fn refuse_tuple_protocol(body: &[Stmt], what: &str, diagnostics: &mut Diagnostics) -> Option<()> {
    let method = body.iter().find_map(|statement| match statement {
        Stmt::FunctionDef { name, .. } if TUPLE_PROTOCOL.contains(&name.name.as_str()) => {
            Some(name)
        }
        _ => None,
    });
    match method {
        Some(method) => {
            diagnostics.push(planned(method.span, &format!("{what} `{}`", method.name)));
            None
        }
        None => Some(()),
    }
}

/// Returns the methods of the named tuple that its class does not define itself, at its base.
fn methods(
    class: &str,
    fields: &[Field],
    hashable: bool,
    defined: &BTreeSet<String>,
    at: Span,
) -> Vec<Stmt> {
    let own = |name: &str| defined.contains(name);
    let mut source = String::new();
    write_init(&mut source, fields);
    if !own("__repr__") {
        write_repr(&mut source, fields);
    }
    let (mine, theirs) = (tuple(fields, "self"), tuple(fields, "other"));
    if !own("__eq__") {
        let _ = write!(
            source,
            "def __eq__(self, other: \"{class}\") -> bool:\n    return {mine} == {theirs}\n"
        );
    }
    // A class that defines `__eq__` and not `__hash__` has no hash, as in CPython.
    if hashable && !own("__hash__") && !own("__eq__") {
        let _ = write!(
            source,
            "def __hash__(self) -> int:\n    return HASH({mine})\n"
        );
    }
    // The order methods serve the runtime, which orders instances held in a list; a
    // comparison the program writes orders the tuples of the fields, and reports fields that
    // do not order.
    if fields.iter().all(|field| orders(&field.annotation)) {
        for (method, operator) in ORDERS.into_iter().filter(|(method, _)| !own(method)) {
            let _ = write!(
                source,
                "def {method}(self, other: \"{class}\") -> bool:\n    return {mine} {operator} \
                 {theirs}\n"
            );
        }
    }
    let _ = write!(source, "def _asdict(self) -> int:\n    return {{");
    for field in fields {
        let _ = write!(source, "'{0}': self.{0}, ", field.name);
    }
    let _ = write!(source, "}}\ndef fields(self) -> int:\n    return {mine}\n");
    let mut methods = parsed(&source, fields, at);
    if let [
        ..,
        Stmt::FunctionDef {
            returns: as_dict, ..
        },
        Stmt::FunctionDef { name, returns, .. },
    ] = methods.as_mut_slice()
    {
        *as_dict = Some(subscript(
            "dict",
            vec![name_expression("str", at), union(fields, at)],
            at,
        ));
        NAMED_TUPLE_FIELDS.clone_into(&mut name.name);
        *returns = Some(fields_annotation(fields, at));
    }
    methods
}

/// Returns the hidden method `_replace` calls for `class`: each field is taken as
/// `tuple[F] | None`, so a field whose value may be `None` is told apart from one the call
/// leaves out, and the new instance is of `class`.
fn replace_method(class: &str, fields: &[Field], at: Span) -> Vec<Stmt> {
    let mut source = String::from("def replace(self");
    for field in fields {
        let _ = write!(source, ", {}: int", field.name);
    }
    let _ = write!(source, ") -> \"{class}\":\n    return {class}(");
    for field in fields {
        let _ = write!(source, "self.{0} if {0} is None else {0}[0], ", field.name);
    }
    source.push_str(")\n");
    let mut methods = parsed(&source, &[], at);
    if let Some(Stmt::FunctionDef {
        name, parameters, ..
    }) = methods.first_mut()
    {
        NAMED_TUPLE_REPLACE.clone_into(&mut name.name);
        for (parameter, field) in parameters.iter_mut().skip(1).zip(fields) {
            let item = subscript("tuple", vec![field.annotation.clone()], at);
            parameter.annotation = Some(Expr::BinOp {
                left: Box::new(item),
                operator: BinaryOperator::BitOr,
                right: Box::new(Expr::Constant {
                    value: Constant::None,
                    span: at,
                }),
                span: at,
            });
        }
    }
    methods
}

/// Returns the union of the fields' types, the type of a value of `_asdict`; `None` for a
/// named tuple without fields, whose dict is empty.
fn union(fields: &[Field], at: Span) -> Expr {
    let mut types = fields.iter().map(|field| field.annotation.clone());
    let Some(first) = types.next() else {
        return Expr::Constant {
            value: Constant::None,
            span: at,
        };
    };
    types.fold(first, |left, right| Expr::BinOp {
        left: Box::new(left),
        operator: BinaryOperator::BitOr,
        right: Box::new(right),
        span: at,
    })
}

/// Returns `name`, read as a name.
fn name_expression(name: &str, at: Span) -> Expr {
    Expr::Name {
        id: name.to_owned(),
        span: at,
    }
}

/// Returns `name[A, B]`, a built-in type given its arguments.
fn subscript(name: &str, mut arguments: Vec<Expr>, at: Span) -> Expr {
    let index = if arguments.len() == 1 {
        arguments.remove(0)
    } else {
        Expr::Tuple {
            elements: arguments,
            span: at,
        }
    };
    Expr::Subscript {
        value: Box::new(name_expression(name, at)),
        index: Box::new(index),
        span: at,
    }
}

/// Returns `tuple[A, B]`, the type of the tuple of the fields.
fn fields_annotation(fields: &[Field], at: Span) -> Expr {
    let items = fields
        .iter()
        .map(|field| field.annotation.clone())
        .collect();
    subscript("tuple", items, at)
}

/// Returns `_fields = ('a', 'b', )`, the names of the fields.
fn fields_statement(fields: &[Field], at: Span) -> Vec<Stmt> {
    let mut source = String::from("_fields = (");
    for field in fields {
        let _ = write!(source, "'{}', ", field.name);
    }
    source.push_str(")\n");
    parsed(&source, &[], at)
}

/// Returns true when the annotation names a type whose values order: a number, a string,
/// `bytes`, or a list or tuple of them.
fn orders(annotation: &Expr) -> bool {
    match annotation {
        Expr::Name { id, .. } => matches!(id.as_str(), "int" | "float" | "bool" | "str" | "bytes"),
        Expr::Subscript { value, index, .. } => {
            let sequence = matches!(value.as_ref(), Expr::Name { id, .. }
                if matches!(id.as_str(), "tuple" | "list" | "Tuple" | "List"));
            let items = match index.as_ref() {
                Expr::Tuple { elements, .. } => elements.iter().collect(),
                item => vec![item],
            };
            sequence
                && items.into_iter().all(|item| {
                    matches!(
                        item,
                        Expr::Constant {
                            value: Constant::Ellipsis,
                            ..
                        }
                    ) || orders(item)
                })
        }
        _ => false,
    }
}

/// Returns the error for what CPython raises when the class statement runs: the exception's
/// class and its text.
fn class_error(span: Span, (kind, text): (&str, &str)) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(404),
        span,
        format!("{text}, as CPython raises `{kind}` when the class statement runs"),
    )
    .with_rule(RuleId::CLS_27)
}

/// Returns the error for a form of named tuple TyPy takes from 1.0 (CLS-41).
fn planned(span: Span, what: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        format!("{what} is planned for TyPy 1.0 (CLS-41)"),
    )
    .with_rule(RuleId::CLS_41)
}
