//! Reads the names of `typing` and `collections.abc` given as the classes of `isinstance` and
//! of class patterns (TYP-17): `Iterable` and `Iterator`, which a class is by the methods it
//! defines; `List`, `Dict`, `Set`, `Tuple` and `Type`, the built-in classes; and `Optional[T]`
//! and `Union[A, B]`, the classes of their members. The forms CPython refuses with `TypeError`
//! are refused when compiling.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};
use typy_source::syntax::{Constant, Expr};

use super::BodyChecker;
use crate::check::{AnnotationName, annotation_name};
use crate::typed::{ClassId, TypedExpr};
use crate::types::{AbstractClass, TypeObjectOf};

impl BodyChecker<'_> {
    /// Returns the classes a name of `typing` or `collections.abc` stands for as the class of
    /// an `isinstance` test, `Ok(None)` when `expression` names none of them, or the error for
    /// a form CPython refuses or answers differently from one version to the next.
    pub(super) fn typing_classes(
        &mut self,
        expression: &Expr,
        held: &mut Vec<TypedExpr>,
    ) -> Result<Option<Vec<TypeObjectOf>>, Option<Box<Diagnostic>>> {
        if let Expr::Subscript { value, index, .. } = expression
            && let Some(name) = self.standard_name(value)
        {
            return match name {
                AnnotationName::Optional | AnnotationName::Union => {
                    self.union_classes(name, index, held).map(Some)
                }
                _ => Err(Some(type_error(
                    expression,
                    "Subscripted generics cannot be used with class and instance checks",
                ))),
            };
        }
        let Some(name) = self.standard_name(expression) else {
            return Ok(None);
        };
        if let AnnotationName::Abstract(collection) = name {
            return Ok(Some(self.abstract_collection_classes(collection)));
        }
        Ok(Some(vec![match name {
            AnnotationName::Iterable(_) => self.abstract_class(AbstractClass::Iterable),
            AnnotationName::Iterator(_) => self.abstract_class(AbstractClass::Iterator),
            AnnotationName::Generator(_) => self.abstract_class(AbstractClass::Generator),
            AnnotationName::Builtin(builtin) => match TypeObjectOf::named(builtin) {
                Some(class) => class,
                None => return Ok(None),
            },
            AnnotationName::Optional | AnnotationName::ClassVar | AnnotationName::Final => {
                let written = match name {
                    AnnotationName::Optional => "Optional",
                    AnnotationName::ClassVar => "ClassVar",
                    _ => "Final",
                };
                return Err(Some(type_error(
                    expression,
                    &format!("typing.{written} cannot be used with isinstance()"),
                )));
            }
            // CPython 3.12 and 3.13 raise `TypeError`, and 3.14 and 3.15 answer `False`.
            AnnotationName::Union => {
                return Err(Some(Box::new(
                    Diagnostic::new(
                        DiagnosticCode::error(202),
                        expression.span(),
                        "`isinstance` with a bare `typing.Union` is refused: CPython 3.12 and \
                         3.13 raise `TypeError`, and 3.14 and 3.15 answer `False`",
                    )
                    .with_rule(RuleId::TYP_17)
                    .with_help("name the classes, as `isinstance(x, (int, str))`"),
                )));
            }
            _ => return Ok(None),
        }]))
    }

    /// Returns the classes `isinstance` tests for an abstract collection (LIB-20, ADR-0086):
    /// the built-in types CPython registers with one that has no `__subclasshook__`, and for
    /// `Container`, `Collection` and `Reversible`, which CPython reads by their methods, those
    /// types and the classes of the program that define the methods.
    fn abstract_collection_classes(
        &self,
        collection: crate::abstract_type::AbstractCollection,
    ) -> Vec<TypeObjectOf> {
        use crate::abstract_type::AbstractCollection;
        use crate::types::View;
        let views = [
            TypeObjectOf::View(View::Keys),
            TypeObjectOf::View(View::Values),
            TypeObjectOf::View(View::Items),
        ];
        match collection {
            AbstractCollection::Sequence => vec![
                TypeObjectOf::List,
                TypeObjectOf::Tuple,
                TypeObjectOf::Str,
                TypeObjectOf::Bytes,
                TypeObjectOf::Range,
            ],
            AbstractCollection::MutableSequence => vec![TypeObjectOf::List],
            AbstractCollection::Mapping | AbstractCollection::MutableMapping => {
                vec![TypeObjectOf::Dict]
            }
            AbstractCollection::Set => vec![
                TypeObjectOf::Set,
                TypeObjectOf::View(View::Keys),
                TypeObjectOf::View(View::Items),
            ],
            AbstractCollection::MutableSet => vec![TypeObjectOf::Set],
            AbstractCollection::KeysView => vec![views[0].clone()],
            AbstractCollection::ValuesView => vec![views[1].clone()],
            AbstractCollection::ItemsView => vec![views[2].clone()],
            AbstractCollection::Container => vec![self.abstract_class(AbstractClass::Container)],
            AbstractCollection::Collection => {
                vec![self.abstract_class(AbstractClass::Collection)]
            }
            AbstractCollection::Reversible => {
                vec![self.abstract_class(AbstractClass::Reversible)]
            }
        }
    }

    /// Returns how a class pattern's class names an alias of `typing`, which is no class, so
    /// CPython raises `TypeError` for the pattern; `None` for anything else (TYP-17, TYP-21).
    pub(super) fn typing_alias(&self, expression: &Expr) -> Option<&'static str> {
        use crate::check::Origin;
        Some(match self.standard_name(expression)? {
            AnnotationName::Iterable(Origin::Typing) => "Iterable",
            AnnotationName::Iterator(Origin::Typing) => "Iterator",
            AnnotationName::Generator(Origin::Typing) => "Generator",
            AnnotationName::Builtin("list") => "List",
            AnnotationName::Builtin("dict") => "Dict",
            AnnotationName::Builtin("set") => "Set",
            AnnotationName::Builtin("tuple") => "Tuple",
            AnnotationName::Builtin(_) => "Type",
            AnnotationName::Optional => "Optional",
            AnnotationName::Union => "Union",
            _ => return None,
        })
    }

    /// Returns the name of `typing` or `collections.abc` an expression names: an imported name,
    /// or one read through its module.
    fn standard_name(&self, expression: &Expr) -> Option<AnnotationName> {
        match expression {
            Expr::Name { id, .. } => self.program.annotation_names.get(id).copied(),
            Expr::Attribute {
                value, attribute, ..
            } => match value.as_ref() {
                Expr::Name { id, .. } => {
                    let module = self.program.annotation_modules.get(id)?;
                    annotation_name(module, &attribute.name)
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Returns the classes of the members of `Optional[T]` or `Union[A, B]`, `None` giving
    /// `NoneType`.
    fn union_classes(
        &mut self,
        name: AnnotationName,
        index: &Expr,
        held: &mut Vec<TypedExpr>,
    ) -> Result<Vec<TypeObjectOf>, Option<Box<Diagnostic>>> {
        let members: Vec<&Expr> = match index {
            Expr::Tuple { elements, .. } => elements.iter().collect(),
            single => vec![single],
        };
        if name == AnnotationName::Optional && members.len() != 1 {
            return Err(Some(Box::new(
                Diagnostic::new(
                    DiagnosticCode::error(408),
                    index.span(),
                    "`Optional` takes one type, as CPython requires",
                )
                .with_rule(RuleId::LIB_17),
            )));
        }
        let mut classes = Vec::new();
        for member in members {
            if let Expr::Constant {
                value: Constant::None,
                ..
            } = member
            {
                classes.push(TypeObjectOf::NoneType);
                continue;
            }
            classes.extend(self.isinstance_classes_holding(member, held)?);
        }
        if name == AnnotationName::Optional {
            classes.push(TypeObjectOf::NoneType);
        }
        Ok(classes)
    }

    /// Returns `Iterable` or `Iterator` with the classes of the program that have its methods,
    /// their own or a base's, each with its bases.
    fn abstract_class(&self, kind: AbstractClass) -> TypeObjectOf {
        let methods: &[&str] = match kind {
            AbstractClass::Iterable => &["__iter__"],
            AbstractClass::Iterator => &["__iter__", "__next__"],
            AbstractClass::Generator => &["__iter__", "__next__", "send", "throw", "close"],
            AbstractClass::Container => &["__contains__"],
            AbstractClass::Collection => &["__len__", "__iter__", "__contains__"],
            AbstractClass::Reversible => &["__reversed__", "__iter__"],
        };
        let implementers = (0..self.program.classes.len())
            .map(|index| ClassId(super::program::id(index)))
            .filter(|class| !self.program.is_protocol(*class))
            .filter(|class| {
                methods
                    .iter()
                    .all(|method| self.program.method(*class, method).is_some())
            })
            .map(|class| (class, self.bases(class)))
            .collect();
        TypeObjectOf::Abstract { kind, implementers }
    }

    /// Returns the bases of a class of the program, nearest first.
    fn bases(&self, class: ClassId) -> Vec<ClassId> {
        let mut found = Vec::new();
        let mut current = self.program.classes[class.0 as usize].base;
        while let Some(base) = current {
            found.push(base);
            current = self.program.classes[base.0 as usize].base;
        }
        found
    }
}

/// Returns the error for an `isinstance` test CPython answers with `TypeError`.
fn type_error(expression: &Expr, message: &str) -> Box<Diagnostic> {
    Box::new(
        Diagnostic::new(
            DiagnosticCode::error(401),
            expression.span(),
            format!("{message}: CPython raises `TypeError`"),
        )
        .with_rule(RuleId::TYP_17),
    )
}
