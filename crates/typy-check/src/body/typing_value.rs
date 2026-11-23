//! Typing values (TYP-18, ADR-0087): a name of `typing` or `collections.abc`, such a name or
//! a built-in class given type arguments, and a union of types written with `|`, read as a
//! value. Each distinct value is a constant that holds the text CPython 3.15 writes and a key
//! that the values CPython finds equal share: a union's is the set of its members' keys, so
//! `Optional[int] == int | None`. Its type notes the Python floor from which every oracle
//! writes it alike, so writing it below that floor is refused, and storing, passing and
//! comparing it are accepted from any floor.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, Constant, Expr};

use super::BodyChecker;
use crate::check::{AnnotationName, Origin};
use crate::typed::{TypedExpr, TypedExprKind};
use crate::types::{Type, TypeObjectOf};

/// The oldest Python floor TyPy accepts, from which most typing values are written alike.
const OLDEST: u8 = Type::TYPING_ALIKE;

/// The floor from which every oracle writes a union as `int | None` (Python 3.14 made
/// `typing.Union` the class of every union).
const UNIONS_ALIKE: u8 = 14;

/// The floor from which every oracle writes a type alias inside a typing value with its module
/// (Python 3.15).
const ALIASES_ALIKE: u8 = 15;

/// One part of a typing value, as CPython writes it and compares it.
#[derive(Clone, Debug)]
struct Part {
    /// How a built-in generic, a `collections.abc` generic or a union writes it.
    text: String,
    /// How a generic alias of `typing` writes it, which writes `None` as `NoneType`.
    typing_text: String,
    /// How it is written alone, where that differs, as `<class 'collections.abc.Iterator'>`.
    alone: Option<String>,
    /// The key the parts CPython finds equal share.
    key: String,
    /// The floor from which every oracle writes it alike.
    floor: u8,
    /// The floor from which every oracle writes it alike alone.
    alone_floor: u8,
    kind: PartKind,
}

/// What a part of a typing value is.
#[derive(Clone, Debug)]
enum PartKind {
    /// A class, an alias or a type parameter, which is no typing value alone.
    Class,
    /// `None`.
    NoneValue,
    /// `...`, or a list of types, as `Callable` takes them, or `()`.
    Argument,
    /// A typing value: a name of `typing` or `collections.abc`, or a generic alias.
    Value,
    /// A union of these members, each one of the other kinds.
    Union(Vec<Part>),
}

impl Part {
    fn new(text: String, key: String, floor: u8, kind: PartKind) -> Self {
        Self {
            typing_text: text.clone(),
            text,
            alone: None,
            key,
            floor,
            alone_floor: floor,
            kind,
        }
    }

    /// Returns a union of `members`, flattened, with each member CPython finds equal to an
    /// earlier one left out; the floor is at least that of a union written with `Optional`
    /// or `Union` when `spelled` holds.
    fn union(members: Vec<Self>, spelled: bool) -> Self {
        let mut flat: Vec<Self> = Vec::new();
        for member in members {
            let each = match member.kind {
                PartKind::Union(inner) => inner,
                _ => vec![member],
            };
            for part in each {
                if flat.iter().all(|known| known.key != part.key) {
                    flat.push(part);
                }
            }
        }
        let text = flat
            .iter()
            .map(|member| member.text.as_str())
            .collect::<Vec<_>>()
            .join(" | ");
        let mut keys: Vec<&str> = flat.iter().map(|member| member.key.as_str()).collect();
        keys.sort_unstable();
        let floor = flat
            .iter()
            .map(|member| member.floor)
            .chain(spelled.then_some(UNIONS_ALIKE))
            .max()
            .unwrap_or(OLDEST);
        let key = format!("{{{}}}", keys.join(","));
        Self::new(text, key, floor, PartKind::Union(flat))
    }

    /// Returns true when the part is a typing value written alone.
    fn is_value(&self) -> bool {
        match &self.kind {
            PartKind::Value => true,
            PartKind::Union(members) => members.len() > 1,
            _ => false,
        }
    }
}

/// The alias whose value is being written: its module and qualified name, which keys its
/// type parameters, and their names.
#[derive(Clone, Debug)]
pub(super) struct AliasRendering {
    pub(super) owner: String,
    pub(super) parameters: Vec<String>,
}

impl BodyChecker<'_> {
    /// Returns true when the program binds `id` where it is read: in the running function or
    /// the module, or, while an alias's value is written, in the alias's module.
    fn bound_here(&self, id: &str, span: Span) -> bool {
        if self.alias_rendering.is_none() {
            return self.binds(id, span);
        }
        self.program.module_symbols.contains_key(id)
            || self.program.function_ids.contains_key(id)
            || self.program.class_ids.contains_key(id)
            || self.program.module_functions.contains_key(id)
    }

    /// Returns true when `id` is a variable of the running function, which no alias's value
    /// reads.
    fn local_here(&self, id: &str) -> bool {
        self.alias_rendering.is_none() && self.locals.contains_key(id)
    }

    /// Returns `expression` as a typing value when it is one: a name of `typing` or
    /// `collections.abc`, such a name or a built-in class given type arguments, or a union of
    /// types written with `|`; `None` for any other expression, which is checked as it is.
    pub(super) fn typing_value(&mut self, expression: &Expr) -> Option<TypedExpr> {
        let candidate = match expression {
            Expr::Name { id, span } => self.typing_name_of(id, *span).is_some(),
            Expr::Attribute { .. } => self.standard_name_of(expression).is_some(),
            Expr::Subscript { value, .. } => self.generic_base(value).is_some(),
            Expr::BinOp {
                operator: BinaryOperator::BitOr,
                ..
            } => true,
            _ => false,
        };
        if !candidate {
            return None;
        }
        if let Some(refused) = self.function_of_typing(expression) {
            return Some(refused);
        }
        if let Some(refused) = self.optional_arity(expression) {
            return Some(refused);
        }
        let part = self.part(expression)?;
        // A union of one class, as `int | int` or `Union[int]`, is that class.
        if let PartKind::Union(members) = &part.kind
            && let [member] = members.as_slice()
            && matches!(member.kind, PartKind::Class)
        {
            return Some(self.expression(leftmost(expression)));
        }
        if !part.is_value() {
            return None;
        }
        let text = part.alone.as_deref().unwrap_or(&part.text);
        let index = self.program.typing_value_object(text, &part.key);
        // CPython keeps one object of each name of `typing` and `collections.abc`.
        if matches!(expression, Expr::Name { .. } | Expr::Attribute { .. }) {
            self.program.single_typing_values.insert(index);
        }
        // A value every oracle from the program's floor writes alike is written freely.
        let floor = if part.alone_floor <= self.program.floor {
            Type::TYPING_ALIKE
        } else {
            part.alone_floor
        };
        Some(Self::typed(
            TypedExprKind::TypingValue(index),
            Type::Typing(floor),
            expression.span(),
        ))
    }

    /// Reports writing `value`, with `print`, `str`, `repr` or an f-string, when it holds a
    /// typing value that an oracle from the program's floor writes otherwise (ADR-0087).
    pub(super) fn check_typing_text(&mut self, value: &TypedExpr) {
        let Some(floor) = typing_floor(&value.ty) else {
            return;
        };
        if floor <= self.program.floor {
            return;
        }
        let message = format!(
            "this typing value is written alike only from Python 3.{floor}, and an older \
             CPython from the program's floor, 3.{}, writes it otherwise",
            self.program.floor
        );
        self.diagnostics.push(
            Diagnostic::new(DiagnosticCode::error(202), value.span, message)
                .with_rule(RuleId::TYP_18)
                .with_help(format!(
                    "set the Python floor to 3.{floor}, or write the value's parts yourself"
                )),
        );
    }

    /// Reports `is` between two typing values other than names of `typing` or
    /// `collections.abc`: CPython keeps one object of some, such as `typing.Iterator[int]`,
    /// and makes others anew, such as `list[int]`, so identity is no rule of the language.
    pub(super) fn typing_identity(&mut self, left: &TypedExpr, right: &TypedExpr, span: Span) {
        let single = |value: &TypedExpr, singles: &std::collections::BTreeSet<u32>| matches!(value.kind, TypedExprKind::TypingValue(index) if singles.contains(&index));
        let singles = &self.program.single_typing_values;
        if single(left, singles) && single(right, singles) {
            return;
        }
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(202),
                span,
                "`is` between typing values other than names of `typing` and `collections.abc` \
                 is refused, since CPython keeps one object of some and makes others anew",
            )
            .with_rule(RuleId::TYP_18)
            .with_help("use `==`, which compares them as CPython does"),
        );
    }

    /// Returns the annotation name the expression names, a name or an attribute of a module
    /// of `typing`, `collections.abc` or `types`, when nothing of the program binds it.
    fn standard_name_of(&self, expression: &Expr) -> Option<AnnotationName> {
        let root = match expression {
            Expr::Name { id, span } => Some((id, *span)),
            Expr::Attribute { value, .. } => root_name(value),
            _ => None,
        }?;
        if self.bound_here(root.0, root.1) {
            return None;
        }
        let scope = super::locals::annotation_scope(self.program);
        crate::check::standard_name(expression, &scope)
    }

    /// Returns the annotation name `id` names alone, when nothing of the program binds it.
    fn typing_name_of(&self, id: &str, span: Span) -> Option<AnnotationName> {
        if self.bound_here(id, span) {
            return None;
        }
        self.program.annotation_names.get(id).copied()
    }

    /// Returns E0202 for `cast`, `overload`, `NamedTuple`, `dataclass`, `field`, `ABC` or
    /// `abstractmethod` read as a value, which TyPy reads only where it is called, decorates or
    /// a class names it as its base.
    fn function_of_typing(&mut self, expression: &Expr) -> Option<TypedExpr> {
        let name = self.standard_name_of(expression)?;
        if !matches!(
            name,
            AnnotationName::Cast
                | AnnotationName::NamedTuple
                | AnnotationName::Dataclasses
                | AnnotationName::Overload
                | AnnotationName::AbstractClasses
        ) {
            return None;
        }
        let span = expression.span();
        let message = "this name of `typing`, `dataclasses` or `abc` is read only where it is \
                       called, decorates or a class names it, not as a value";
        Some(
            self.error(
                Diagnostic::new(DiagnosticCode::error(202), span, message)
                    .with_rule(RuleId::TYP_18)
                    .with_help("call it where you need it"),
                span,
            ),
        )
    }

    /// Returns the module and name of the alias of a module `id` names, its own or one it
    /// imports, when nothing of the running function binds it.
    pub(super) fn module_alias(&self, id: &str, span: Span) -> Option<(String, String)> {
        if self.local_here(id) || (self.alias_rendering.is_none() && self.binds(id, span)) {
            return None;
        }
        if let Some(alias) = self.program.imported_aliases.get(id) {
            return Some((alias.module.clone(), alias.name.clone()));
        }
        self.program
            .type_aliases
            .contains(id)
            .then(|| (self.program.module.clone(), id.to_owned()))
    }

    /// Returns how many type parameters the alias `name` of `module` has.
    fn alias_parameters(&self, module: &str, name: &str) -> Option<usize> {
        self.program
            .alias_sources
            .get(&(module.to_owned(), name.to_owned()))
            .map(|source| source.parameters.len())
    }

    /// Returns E0408 for `Optional` given other than one type, which CPython refuses.
    fn optional_arity(&mut self, expression: &Expr) -> Option<TypedExpr> {
        let Expr::Subscript { value, index, .. } = expression else {
            return None;
        };
        if !matches!(self.generic_base(value), Some(Base::Optional))
            || !matches!(index.as_ref(), Expr::Tuple { .. })
        {
            return None;
        }
        Some(
            self.error(
                Diagnostic::new(
                    DiagnosticCode::error(408),
                    index.span(),
                    "`Optional` takes one type, as CPython requires",
                )
                .with_rule(RuleId::LIB_17),
                expression.span(),
            ),
        )
    }

    /// Returns what the base of a subscript is, when a typing value may give it type
    /// arguments: a built-in class, or a name of `typing` or `collections.abc`.
    fn generic_base(&self, value: &Expr) -> Option<Base> {
        if let Expr::Name { id, span } = value
            && !self.bound_here(id, *span)
            && matches!(id.as_str(), "list" | "dict" | "set" | "tuple" | "type")
        {
            return Some(Base::Builtin(id.clone()));
        }
        if let Expr::Name { id, span } = value
            && let Some((module, name)) = self.module_alias(id, *span)
        {
            return Some(Base::Alias { module, name });
        }
        let name = self.standard_name_of(value)?;
        Some(match name {
            AnnotationName::Optional => Base::Optional,
            AnnotationName::Union => Base::Union,
            other => Base::Named(named_text(other)?),
        })
    }

    /// Returns the part of a typing value `expression` is, or `None` when it is no type.
    fn part(&mut self, expression: &Expr) -> Option<Part> {
        match expression {
            Expr::Constant {
                value: Constant::None,
                ..
            } => Some(Part {
                typing_text: "NoneType".to_owned(),
                ..Part::new(
                    "None".to_owned(),
                    "NoneType".to_owned(),
                    OLDEST,
                    PartKind::NoneValue,
                )
            }),
            Expr::Constant {
                value: Constant::Ellipsis,
                ..
            } => Some(Part::new(
                "...".to_owned(),
                "...".to_owned(),
                OLDEST,
                PartKind::Argument,
            )),
            Expr::List { elements, .. } => {
                let parts = self.parts(elements)?;
                let written = |text: fn(&Part) -> &str| {
                    format!(
                        "[{}]",
                        parts.iter().map(text).collect::<Vec<_>>().join(", ")
                    )
                };
                Some(Part {
                    typing_text: written(typing_text_of),
                    ..Part::new(
                        written(text_of),
                        written(key_of),
                        parts.iter().map(|part| part.floor).max().unwrap_or(OLDEST),
                        PartKind::Argument,
                    )
                })
            }
            Expr::Tuple { elements, .. } if elements.is_empty() => Some(Part::new(
                "()".to_owned(),
                "()".to_owned(),
                OLDEST,
                PartKind::Argument,
            )),
            Expr::Name { id, span } => self.named_part(id, *span),
            Expr::Attribute { .. } => self.standard_name_of(expression).and_then(name_part),
            Expr::Subscript { value, index, .. } => self.subscript_part(value, index),
            Expr::BinOp {
                left,
                operator: BinaryOperator::BitOr,
                right,
                ..
            } => {
                let left = self.part(left)?;
                let right = self.part(right)?;
                if matches!(left.kind, PartKind::Argument)
                    || matches!(right.kind, PartKind::Argument)
                {
                    return None;
                }
                Some(Part::union(vec![left, right], false))
            }
            _ => None,
        }
    }

    /// Adds the values `Literal[...]` writes in `written` to `values`, flattening a nested
    /// `Literal[...]` and leaving out a value written before, as CPython does; `None` when
    /// one is no constant `Literal` takes.
    fn literal_values(&self, written: &[Expr], values: &mut Vec<String>) -> Option<()> {
        for each in written {
            let text = match each {
                Expr::Constant {
                    value: Constant::None,
                    ..
                } => "None".to_owned(),
                Expr::Subscript { value, index, .. }
                    if self.standard_name_of(value) == Some(AnnotationName::Literal) =>
                {
                    let inner: Vec<Expr> = match index.as_ref() {
                        Expr::Tuple { elements, .. } => elements.clone(),
                        other => vec![other.clone()],
                    };
                    self.literal_values(&inner, values)?;
                    continue;
                }
                other => crate::literal_type::LiteralValue::of_written(other)?.to_string(),
            };
            if !values.contains(&text) {
                values.push(text);
            }
        }
        Some(())
    }

    /// Returns the parts of each of `elements`, or `None` when one is no type.
    fn parts(&mut self, elements: &[Expr]) -> Option<Vec<Part>> {
        elements.iter().map(|each| self.part(each)).collect()
    }

    /// Returns the part a name is: a name of `typing` or `collections.abc`, a built-in class,
    /// a class or a type alias of the program, or a type parameter of the running function.
    fn named_part(&mut self, id: &str, span: Span) -> Option<Part> {
        if let Some(name) = self.typing_name_of(id, span) {
            return name_part(name);
        }
        // An alias's type parameter is written by its name, keyed by the alias.
        if let Some(rendering) = &self.alias_rendering
            && rendering.parameters.iter().any(|parameter| parameter == id)
        {
            return Some(Part::new(
                id.to_owned(),
                format!("typevar:alias:{}.{id}", rendering.owner),
                OLDEST,
                PartKind::Class,
            ));
        }
        // A type parameter is written by its name in every copy (GEN-03).
        if self.alias_rendering.is_none()
            && self.program.type_arguments.contains_key(id)
            && !self.binds(id, span)
        {
            let owner = without_type_arguments(&self.qualified_name);
            return Some(Part::new(
                id.to_owned(),
                format!("typevar:{owner}.{id}"),
                OLDEST,
                PartKind::Class,
            ));
        }
        if let Some(class) = self.program.class_ids.get(id).copied()
            && !self.local_here(id)
        {
            let module = &self.program.class_modules[class.0 as usize];
            let name = &self.program.classes[class.0 as usize].name;
            let text = format!("{module}.{name}");
            return Some(Part::new(text.clone(), text, OLDEST, PartKind::Class));
        }
        let alias = if let Some(alias) = self.program.imported_aliases.get(id) {
            Some((alias.module.clone(), alias.name.clone()))
        } else if self.program.type_aliases.contains(id) && !self.local_here(id) {
            Some((self.program.module.clone(), id.to_owned()))
        } else {
            None
        };
        if let Some((module, name)) = alias {
            return Some(Part::new(
                format!("{module}.{name}"),
                format!("alias:{module}.{name}"),
                ALIASES_ALIKE,
                PartKind::Class,
            ));
        }
        if self.bound_here(id, span) {
            return None;
        }
        TypeObjectOf::named(id).map(|class| {
            let text = class.name().to_owned();
            Part::new(text.clone(), text, OLDEST, PartKind::Class)
        })
    }

    /// Returns the part `base[index]` is: a generic alias of a built-in class, of `typing` or
    /// of `collections.abc`, or a union written with `Optional` or `Union`.
    fn subscript_part(&mut self, base: &Expr, index: &Expr) -> Option<Part> {
        let base = self.generic_base(base)?;
        let written: Vec<Expr> = match index {
            Expr::Tuple { elements, .. } if !elements.is_empty() => elements.clone(),
            other => vec![other.clone()],
        };
        // `Literal[...]` takes constants, not types (TYP-28).
        if let Base::Named(NamedText {
            module: "typing",
            name: "Literal",
            ..
        }) = base
        {
            let mut values = Vec::new();
            self.literal_values(&written, &mut values)?;
            return Some(literal_part(&values));
        }
        let arguments = self.parts(&written)?;
        match base {
            Base::Optional => {
                let [argument] = <[Part; 1]>::try_from(arguments).ok()?;
                let none = self.part(&Expr::Constant {
                    value: Constant::None,
                    span: index.span(),
                })?;
                Some(Part::union(vec![argument, none], true))
            }
            Base::Union => Some(Part::union(arguments, true)),
            Base::Builtin(name) => Some(generic(&name, &arguments, text_of)),
            // A generic alias given type arguments, as many as written, as CPython takes them,
            // is written with its module only from 3.15.
            Base::Alias { module, name } => {
                if self.alias_parameters(&module, &name)? == 0 {
                    return None;
                }
                let mut part = generic(&format!("{module}.{name}"), &arguments, text_of);
                part.key = format!("alias:{}", part.key);
                part.floor = ALIASES_ALIKE;
                part.alone_floor = ALIASES_ALIKE;
                Some(part)
            }
            Base::Named(named) => {
                let prefix = format!("{}.{}", named.module, named.name);
                let text = if named.module == "typing" {
                    typing_text_of
                } else {
                    text_of
                };
                Some(generic(&prefix, &arguments, text))
            }
        }
    }
}

/// What a subscript of a typing value gives type arguments to.
enum Base {
    /// A built-in class, such as `list`.
    Builtin(String),
    /// A type alias of a module, by its module and name.
    Alias { module: String, name: String },
    /// `typing.Optional`.
    Optional,
    /// `typing.Union`.
    Union,
    /// A name of `typing` or `collections.abc`.
    Named(NamedText),
}

/// A name of `typing`, `collections.abc` or `types`, with how it is written.
#[derive(Clone, Copy, Debug)]
struct NamedText {
    module: &'static str,
    name: &'static str,
    /// Whether it is a class, which `repr` writes as `<class '...'>` alone.
    class: bool,
}

/// Returns the module and name an annotation name has, or `None` for one that is no type.
fn named_text(name: AnnotationName) -> Option<NamedText> {
    let typing = |name| NamedText {
        module: "typing",
        name,
        class: false,
    };
    let of = |origin: Origin, name| match origin {
        Origin::Typing => typing(name),
        Origin::Abc => NamedText {
            module: "collections.abc",
            name,
            class: true,
        },
    };
    Some(match name {
        AnnotationName::Iterator(origin) => of(origin, "Iterator"),
        AnnotationName::Iterable(origin) => of(origin, "Iterable"),
        AnnotationName::Generator(origin) => of(origin, "Generator"),
        AnnotationName::Callable(origin) => of(origin, "Callable"),
        AnnotationName::Abstract(crate::abstract_type::AbstractCollection::Set, Origin::Typing) => {
            typing("AbstractSet")
        }
        AnnotationName::Abstract(collection, origin) => of(origin, collection.name()),
        AnnotationName::Builtin(builtin) => typing(match builtin {
            "list" => "List",
            "dict" => "Dict",
            "set" => "Set",
            "tuple" => "Tuple",
            _ => "Type",
        }),
        AnnotationName::ClassVar => typing("ClassVar"),
        AnnotationName::Final => typing("Final"),
        AnnotationName::Literal => typing("Literal"),
        AnnotationName::SelfType => typing("Self"),
        AnnotationName::Guard(kind) => typing(kind.name()),
        AnnotationName::Optional => typing("Optional"),
        AnnotationName::Union => NamedText {
            module: "typing",
            name: "Union",
            class: true,
        },
        AnnotationName::Protocol => NamedText {
            module: "typing",
            name: "Protocol",
            class: true,
        },
        AnnotationName::TracebackType => NamedText {
            module: "builtins",
            name: "traceback",
            class: true,
        },
        AnnotationName::ModuleType => NamedText {
            module: "builtins",
            name: "module",
            class: true,
        },
        AnnotationName::Cast
        | AnnotationName::NamedTuple
        | AnnotationName::Dataclasses
        | AnnotationName::Overload
        | AnnotationName::AbstractClasses
        | AnnotationName::Rebound => return None,
    })
}

/// Returns the part a name of `typing`, `collections.abc` or `types` is alone: written with
/// its module, or as `<class '...'>` alone for a class, which `typing.Union` is only from
/// Python 3.14.
fn name_part(name: AnnotationName) -> Option<Part> {
    let union = name == AnnotationName::Union;
    let named = named_text(name)?;
    let text = if named.module == "builtins" {
        named.name.to_owned()
    } else {
        format!("{}.{}", named.module, named.name)
    };
    Some(Part {
        alone: named.class.then(|| format!("<class '{text}'>")),
        alone_floor: if union { UNIONS_ALIKE } else { OLDEST },
        ..Part::new(text.clone(), text, OLDEST, PartKind::Value)
    })
}

/// Returns the generic alias `prefix[arguments]`, each argument written by `text`, which a
/// generic alias of `typing` takes as it writes `None`.
fn generic(prefix: &str, arguments: &[Part], text: fn(&Part) -> &str) -> Part {
    let written: Vec<&str> = arguments.iter().map(text).collect();
    let keys: Vec<&str> = arguments.iter().map(|part| part.key.as_str()).collect();
    let floor = arguments
        .iter()
        .map(|part| part.floor)
        .max()
        .unwrap_or(OLDEST);
    Part::new(
        format!("{prefix}[{}]", written.join(", ")),
        format!("{prefix}[{}]", keys.join(", ")),
        floor,
        PartKind::Value,
    )
}

/// Returns `typing.Literal[...]` of the values, written as `repr` writes them; CPython finds
/// two equal whatever the order of their values (TYP-28).
fn literal_part(values: &[String]) -> Part {
    let mut keys: Vec<&str> = values.iter().map(String::as_str).collect();
    keys.sort_unstable();
    Part::new(
        format!("typing.Literal[{}]", values.join(", ")),
        format!("typing.Literal{{{}}}", keys.join(", ")),
        OLDEST,
        PartKind::Value,
    )
}

/// Returns how a built-in generic, a `collections.abc` generic or a union writes `part`.
fn text_of(part: &Part) -> &str {
    &part.text
}

/// Returns how a generic alias of `typing` writes `part`.
fn typing_text_of(part: &Part) -> &str {
    &part.typing_text
}

/// Returns the key of `part`.
fn key_of(part: &Part) -> &str {
    &part.key
}

/// Returns a function's qualified name without the type arguments of the copy it is, as
/// `pair[int]` gives `pair`, so every copy names its type parameters alike.
pub(super) fn without_type_arguments(name: &str) -> String {
    let mut shown = String::with_capacity(name.len());
    let mut depth = 0_usize;
    for character in name.chars() {
        match character {
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => shown.push(character),
            _ => {}
        }
    }
    shown
}

/// Returns the first member a union written with `|`, `Union[...]` or `Optional[...]` names.
fn leftmost(expression: &Expr) -> &Expr {
    match expression {
        Expr::BinOp {
            left,
            operator: BinaryOperator::BitOr,
            ..
        } => leftmost(left),
        Expr::Subscript { index, .. } => match index.as_ref() {
            Expr::Tuple { elements, .. } if !elements.is_empty() => leftmost(&elements[0]),
            single => leftmost(single),
        },
        other => other,
    }
}

/// Returns the name an attribute chain starts from, with its span.
fn root_name(expression: &Expr) -> Option<(&String, Span)> {
    match expression {
        Expr::Name { id, span } => Some((id, *span)),
        Expr::Attribute { value, .. } => root_name(value),
        _ => None,
    }
}

/// Returns the floor from which every oracle writes a value of `ty` alike, when it holds a
/// typing value, itself or in a collection.
fn typing_floor(ty: &Type) -> Option<u8> {
    match ty {
        Type::Typing(floor) | Type::TypeAlias(floor) => Some(*floor),
        Type::List(item) | Type::Set(item) | Type::TupleOf(item) | Type::Optional(item) => {
            typing_floor(item)
        }
        Type::Dict(key, value) => typing_floor(key).max(typing_floor(value)),
        Type::Tuple(items) | Type::Union(items) => items.iter().filter_map(typing_floor).max(),
        _ => None,
    }
}
