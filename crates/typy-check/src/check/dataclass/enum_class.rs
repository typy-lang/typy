//! `class Color(Enum)` (CLS-44, ADR-0093, ADR-0116): a class derived from `enum.Enum` becomes
//! a class whose members are class variables, each an instance made where the class statement
//! runs, with the methods CPython's `EnumType` and `Enum` give it written as source the checker
//! reads as if the program had written it: `__init__`, the `name` and `value` properties,
//! `__repr__` and `__str__`, and three hidden static methods, which find a member by its
//! value or its name and list the members, and which the checker calls for `Color(value)`,
//! `Color[name]`, a loop over the class, `len` and `in`. What CPython raises when the class
//! statement runs is reported with its text.

use std::fmt::Write as _;

use camino::Utf8PathBuf;
use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, FileId, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Identifier, Stmt};
use typy_source::{PythonFloor, SourceFile, parse};

use super::{DataclassInfo, Dataclasses, Names};
use crate::class_type::{ENUM_LOOKUP, ENUM_MEMBER, ENUM_MEMBERS, ENUM_NAMED, ENUM_VALUE};
use crate::literal_type::LiteralValue;

/// The names of the methods the expansion writes, which a member may not take.
const WRITTEN: [&str; 5] = ["name", "value", "_name_", "_value_", "__init__"];

/// The kinds of enum class TyPy gives: `Enum`, and `IntEnum` and `StrEnum`, whose members are
/// also their `int` or `str` values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Plain,
    Int,
    Str,
}

impl Kind {
    fn named(name: &str) -> Option<Self> {
        match name {
            "Enum" => Some(Self::Plain),
            "IntEnum" => Some(Self::Int),
            "StrEnum" => Some(Self::Str),
            _ => None,
        }
    }
}

/// One member of an enum class, in the order the class body defines it.
struct Member {
    name: Identifier,
    value: LiteralValue,
    /// Where the value is written, for a constant; `None` for `auto()`.
    written: Option<Expr>,
    /// The earlier member whose value equals this one's, which this one names (an alias).
    alias_of: Option<usize>,
}

impl Dataclasses {
    /// Gives a class derived from `enum.Enum` its members and methods; returns its name and
    /// what the checker needs of it, and `None` for any other statement and after a reported
    /// problem.
    pub(super) fn expand_enum(
        &mut self,
        statement: &mut Stmt,
        module: &str,
        names: &Names,
        diagnostics: &mut Diagnostics,
    ) -> Option<(String, DataclassInfo)> {
        let Stmt::ClassDef {
            name,
            bases,
            keyword_count,
            body,
            decorators,
            type_parameters,
            ..
        } = statement
        else {
            return None;
        };
        if let Some((base, has_members)) = bases.iter().find_map(|base| {
            let key = names.class_key(base, module)?;
            Some((key.1.clone(), *self.enums.get(&key)?))
        }) {
            diagnostics.push(if has_members {
                class_error(
                    name.span,
                    &format!("<enum '{}'> cannot extend <enum '{base}'>", name.name),
                )
            } else {
                planned(
                    name.span,
                    "a class derived from an enum class without members",
                )
            });
            bases.clear();
            return None;
        }
        let kind = bases
            .iter()
            .find_map(|base| match names.class_key(base, module) {
                Some((from, kind)) if from == "enum" => Some(kind),
                _ => None,
            })?;
        let at = name.span;
        let Some(kind) = Kind::named(&kind) else {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    at,
                    format!("a class derived from `enum.{kind}` is planned for TyPy 0.12 (CLS-44)"),
                )
                .with_rule(RuleId::CLS_44),
            );
            bases.clear();
            return None;
        };
        let refused = if bases.len() > 1 || *keyword_count > 0 {
            Some("an enum class with other bases or keywords".to_owned())
        } else if !decorators.is_empty() || !type_parameters.is_empty() {
            Some("an enum class with decorators or type parameters".to_owned())
        } else {
            None
        };
        if let Some(refused) = refused {
            diagnostics.push(planned(at, &refused));
            // The base is reported here, so the class check does not report it again.
            bases.clear();
            return None;
        }
        let members = members(body, (module, names, kind), diagnostics)?;
        bases.clear();
        let class = name.name.clone();
        self.enums
            .insert((module.to_owned(), class.clone()), !members.is_empty());
        let mut expanded: Vec<Stmt> = Vec::with_capacity(body.len() + members.len() + 8);
        expanded.extend(member_variables(&class, &members, at));
        expanded.append(body);
        expanded.extend(methods(&class, (&members, kind), at));
        *body = expanded;
        Some((class, DataclassInfo::default()))
    }
}

/// Takes the members out of an enum class's body, in order, and returns them; reports a
/// member CPython refuses, and a form TyPy takes from a later release.
fn members(
    body: &mut Vec<Stmt>,
    (module, names, kind): (&str, &Names, Kind),
    diagnostics: &mut Diagnostics,
) -> Option<Vec<Member>> {
    let mut members: Vec<Member> = Vec::new();
    let mut failed = false;
    for statement in std::mem::take(body) {
        let Stmt::Assign { targets, value, .. } = &statement else {
            if let Some(refused) = refused_statement(&statement) {
                diagnostics.push(refused);
                failed = true;
            }
            body.push(statement);
            continue;
        };
        let [Expr::Name { id, span }] = targets.as_slice() else {
            body.push(statement);
            continue;
        };
        if id.starts_with("__") && id.ends_with("__") {
            body.push(statement);
            continue;
        }
        let name = Identifier {
            name: id.clone(),
            span: *span,
        };
        match member(name, value, &members, (module, names, kind)) {
            Ok(member) => members.push(member),
            Err(diagnostic) => {
                diagnostics.push(*diagnostic);
                failed = true;
            }
        }
    }
    (!failed).then_some(members)
}

/// Returns the member `name = value`, after the members before it, or why it is refused.
fn member(
    name: Identifier,
    value: &Expr,
    before: &[Member],
    (module, names, kind): (&str, &Names, Kind),
) -> Result<Member, Box<Diagnostic>> {
    if let Some(earlier) = before.iter().find(|each| each.name.name == name.name) {
        return Err(Box::new(class_error(
            name.span,
            &format!("'{}' already defined as {}", name.name, earlier.value),
        )));
    }
    if name.name.starts_with('_') && name.name.ends_with('_')
        || WRITTEN.contains(&name.name.as_str())
    {
        return Err(Box::new(planned(
            name.span,
            &format!("an enum member named `{}`", name.name),
        )));
    }
    let is_auto = matches!(value, Expr::Call { function, arguments, keywords, .. }
        if arguments.is_empty() && keywords.is_empty()
            && names.class_key(function, module) == Some(("enum".to_owned(), "auto".to_owned())));
    let (value, written) = if is_auto && kind == Kind::Str {
        // `StrEnum`'s `auto()` gives the member's name in lower case.
        (LiteralValue::Str(name.name.to_lowercase()), None)
    } else if is_auto {
        (next_value(before, name.span)?, None)
    } else {
        let found = LiteralValue::of_written(value).ok_or_else(|| {
            Box::new(planned(
                value.span(),
                "an enum member whose value is not an `int`, `str`, `bytes` or `bool` constant \
                 or `auto()`",
            ))
        })?;
        (found, Some(value.clone()))
    };
    match (kind, &value) {
        (Kind::Int, LiteralValue::Int(_))
        | (Kind::Str, LiteralValue::Str(_) | LiteralValue::StrPoints(_))
        | (Kind::Plain, _) => {}
        (Kind::Str, other) => {
            return Err(Box::new(class_error(
                name.span,
                &format!("{other} is not a string"),
            )));
        }
        (Kind::Int, _) => {
            return Err(Box::new(planned(
                name.span,
                "an `IntEnum` member whose value is not an `int` constant",
            )));
        }
    }
    let alias_of = before
        .iter()
        .position(|each| each.alias_of.is_none() && each.value.equals(&value));
    Ok(Member {
        name,
        value,
        written,
        alias_of,
    })
}

/// Returns the value `auto()` gives after the members `before`: one more than the largest,
/// or `1` for the first. After a value that is not an `int`, CPython 3.12 gives `1` with a
/// warning and later versions raise `TypeError`, so TyPy refuses it.
fn next_value(before: &[Member], span: Span) -> Result<LiteralValue, Box<Diagnostic>> {
    let mut largest: Option<i128> = None;
    for member in before {
        let number = match &member.value {
            LiteralValue::Int(digits) => digits.parse::<i128>().ok(),
            LiteralValue::Bool(flag) => Some(i128::from(*flag)),
            _ => None,
        };
        let Some(number) = number else {
            return Err(Box::new(
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    span,
                    format!(
                        "`auto()` after the value {} gives `1` with a warning before Python 3.13 \
                         and raises `TypeError` from 3.13",
                        member.value
                    ),
                )
                .with_rule(RuleId::CLS_44)
                .with_help("write the member's value"),
            ));
        };
        largest = Some(largest.map_or(number, |largest| largest.max(number)));
    }
    let next = largest.map_or(Some(1), |largest| largest.checked_add(1));
    next.map(|next| LiteralValue::Int(next.to_string()))
        .ok_or_else(|| Box::new(planned(span, "`auto()` after a value beyond 128 bits")))
}

/// Returns the error for a statement an enum class body may not hold in TyPy 0.12: a method
/// CPython's enum machinery calls when it makes or looks up members, and an annotated
/// assignment, which CPython makes a member.
fn refused_statement(statement: &Stmt) -> Option<Diagnostic> {
    match statement {
        Stmt::FunctionDef { name, .. }
            if matches!(
                name.name.as_str(),
                "__init__"
                    | "__new__"
                    | "_missing_"
                    | "_generate_next_value_"
                    | "__init_subclass__"
            ) =>
        {
            Some(planned(
                name.span,
                &format!("`{}` in an enum class", name.name),
            ))
        }
        Stmt::FunctionDef { name, .. } if WRITTEN.contains(&name.name.as_str()) => Some(planned(
            name.span,
            &format!("a method named `{}` in an enum class", name.name),
        )),
        Stmt::AnnAssign { span, .. } => Some(planned(*span, "an annotated enum member")),
        _ => None,
    }
}

/// Returns the class variable of each member: a new instance for a member of its own, and
/// the variable of the member an alias names.
fn member_variables(class: &str, members: &[Member], at: Span) -> Vec<Stmt> {
    members
        .iter()
        .map(|member| {
            let span = member.name.span;
            let value = match member.alias_of {
                Some(canonical) => Expr::Name {
                    id: crate::check::class::class_value_variable(
                        class,
                        &members[canonical].name.name,
                    ),
                    span,
                },
                None => Expr::Call {
                    function: Box::new(Expr::Name {
                        id: format!("{ENUM_MEMBER}{class}"),
                        span: at,
                    }),
                    arguments: vec![
                        Expr::Constant {
                            value: Constant::Str(member.name.name.clone()),
                            span,
                        },
                        member
                            .written
                            .clone()
                            .unwrap_or_else(|| written_value(&member.value, span)),
                    ],
                    keywords: Vec::new(),
                    span,
                },
            };
            Stmt::Assign {
                targets: vec![Expr::Name {
                    id: member.name.name.clone(),
                    span,
                }],
                value,
                span,
            }
        })
        .collect()
}

/// Returns a constant `auto()` gives, written as an expression.
fn written_value(value: &LiteralValue, span: Span) -> Expr {
    let constant = match value {
        LiteralValue::Int(digits) => Constant::Int(digits.clone()),
        LiteralValue::Bool(flag) => Constant::Bool(*flag),
        LiteralValue::Str(text) => Constant::Str(text.clone()),
        LiteralValue::StrPoints(points) => Constant::StrPoints(points.clone()),
        LiteralValue::Bytes(bytes) => Constant::Bytes(bytes.clone()),
    };
    Expr::Constant {
        value: constant,
        span,
    }
}

/// Returns the type a member's `value` has: the union of its members' value types.
fn value_annotation(members: &[Member]) -> String {
    let mut types: Vec<&str> = Vec::new();
    for member in members {
        let name = match member.value {
            LiteralValue::Int(_) => "int",
            LiteralValue::Bool(_) => "bool",
            LiteralValue::Str(_) | LiteralValue::StrPoints(_) => "str",
            LiteralValue::Bytes(_) => "bytes",
        };
        if !types.contains(&name) {
            types.push(name);
        }
    }
    if types.is_empty() {
        "int".to_owned()
    } else {
        types.join(" | ")
    }
}

/// Returns the methods an enum class gets, as CPython's `Enum` and `EnumType` behave. A member
/// of an `IntEnum` or a `StrEnum` shows, hashes and tests true as its value, and has the
/// hidden method `ENUM_VALUE` that marks it as one the checker reads as its value.
fn methods(class: &str, (members, kind): (&[Member], Kind), at: Span) -> Vec<Stmt> {
    let value = match kind {
        Kind::Plain => value_annotation(members),
        Kind::Int => "int".to_owned(),
        Kind::Str => "str".to_owned(),
    };
    let shown = match kind {
        Kind::Plain => format!("'{class}.' + self._name_"),
        Kind::Int => "str(self._value_)".to_owned(),
        Kind::Str => "self._value_".to_owned(),
    };
    let mut source = format!(
        "def __init__(self, _name_: str, _value_: {value}) -> None:\n    \
         self._name_ = _name_\n    self._value_ = _value_\n\
         @property\ndef name(self) -> str:\n    return self._name_\n\
         @property\ndef value(self) -> {value}:\n    return self._value_\n\
         def __repr__(self) -> str:\n    \
         return '<{class}.' + self._name_ + ': ' + repr(self._value_) + '>'\n\
         def __str__(self) -> str:\n    return {shown}\n\
         @staticmethod\ndef _lookup_(value: {value}) -> '{class}':\n"
    );
    let canonical: Vec<&Member> = members
        .iter()
        .filter(|member| member.alias_of.is_none())
        .collect();
    for member in &canonical {
        let _ = write!(
            source,
            "    if value == {}:\n        return {class}.{}\n",
            member.value, member.name.name
        );
    }
    let _ = write!(
        source,
        "    raise ValueError(repr(value) + ' is not a valid {class}')\n\
         @staticmethod\ndef _named_(name: str) -> '{class}':\n"
    );
    for member in members {
        let _ = write!(
            source,
            "    if name == '{0}':\n        return {class}.{0}\n",
            member.name.name
        );
    }
    let listed: Vec<String> = canonical
        .iter()
        .map(|member| format!("{class}.{}", member.name.name))
        .collect();
    let _ = write!(
        source,
        "    raise KeyError(name)\n\
         @staticmethod\ndef _members_() -> 'list[{class}]':\n    return [{}]\n",
        listed.join(", ")
    );
    if kind != Kind::Plain {
        let _ = write!(
            source,
            "def __lt__(self, other: '{class}') -> bool:\n    return self._value_ < other._value_\n\
             def __le__(self, other: '{class}') -> bool:\n    return self._value_ <= other._value_\n\
             def __gt__(self, other: '{class}') -> bool:\n    return self._value_ > other._value_\n\
             def __ge__(self, other: '{class}') -> bool:\n    return self._value_ >= other._value_\n\
             def __hash__(self) -> int:\n    return hash(self._value_)\n\
             def __bool__(self) -> bool:\n    return bool(self._value_)\n\
             def _value_of_(self) -> {value}:\n    return self._value_\n"
        );
    }
    let file = SourceFile::from_text(FileId(u32::MAX), Utf8PathBuf::from("<enum>"), &source);
    // The source is written above and always parses.
    let mut methods = parse(&file, PythonFloor::default())
        .map(|tree| tree.body)
        .unwrap_or_default();
    for method in &mut methods {
        method.relocate(at);
        if let Stmt::FunctionDef { name, .. } = method {
            let hidden = match name.name.as_str() {
                "_lookup_" => ENUM_LOOKUP,
                "_named_" => ENUM_NAMED,
                "_members_" => ENUM_MEMBERS,
                "_value_of_" => ENUM_VALUE,
                "__hash__" => {
                    hash_by_value(method);
                    continue;
                }
                _ => continue,
            };
            hidden.clone_into(&mut name.name);
        }
    }
    methods
}

/// Makes `__hash__` call the hash a set and a dict take of the value; no program names that
/// function, since `hash` is not provided.
fn hash_by_value(method: &mut Stmt) {
    if let Stmt::FunctionDef { body, .. } = method
        && let [
            Stmt::Return {
                value: Some(Expr::Call { function, .. }),
                ..
            },
        ] = body.as_mut_slice()
        && let Expr::Name { id, .. } = function.as_mut()
    {
        "<hash>".clone_into(id);
    }
}

/// Returns the error for what CPython raises when an enum class statement runs.
fn class_error(span: Span, text: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(404),
        span,
        format!("{text}, as CPython raises `TypeError` when the class statement runs"),
    )
    .with_rule(RuleId::CLS_44)
}

/// Returns the error for a form of enum class TyPy takes from 0.13 (CLS-47).
fn planned(span: Span, what: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        format!("{what} is planned for TyPy 0.13 (CLS-47)"),
    )
    .with_rule(RuleId::CLS_47)
}
