//! Checks what a program does with `ctypes` classes and objects: makes a class with
//! `POINTER(T)` and `T * length`, makes an object by calling a class or `pointer`, reads and
//! assigns its `value`, `contents` and items, measures it with `len` and `sizeof`, and passes
//! it by reference (FFI-03, ADR-0048).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Keyword};

use super::BodyChecker;
use super::builtin_module::{BuiltinName, ModuleFunction};
use crate::check::ctypes_declarations::pointer_of_pointer;
use crate::ctypes::{CData, CType, CtypesFunction, StructureClass};
use crate::typed::{AssignTarget, CDataExpr, CDataPlace, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

/// Returns the error for a use TyPy plans for 0.13 (FFI-10).
fn planned(span: Span, what: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        format!("{what} is planned for TyPy 0.13 (FFI-10)"),
    )
    .with_rule(RuleId::FFI_10)
}

/// Returns the error for a `ctypes` use whose types do not fit, as CPython's `TypeError`.
fn mismatch(code: u16, span: Span, message: String) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::error(code), span, message).with_rule(RuleId::FFI_03)
}

impl BodyChecker<'_> {
    /// Returns `class`, such as `ctypes.c_int`, as a value, which holds nothing.
    pub(super) fn cdata_class(class: CData, span: Span) -> TypedExpr {
        Self::typed(
            TypedExprKind::CData(CDataExpr::Class),
            Type::CDataClass(class),
            span,
        )
    }

    /// Returns `value` unless it is a `ctypes` class, which a program calls, multiplies,
    /// binds to a name or names in a declaration, and uses no other way until 1.0.
    pub(super) fn refuse_class(&mut self, value: TypedExpr) -> TypedExpr {
        if let Type::CDataClass(class) = &value.ty {
            let what = format!("the class `{}` used as a value", class.class_name());
            return self.error(planned(value.span, &what), value.span);
        }
        value
    }

    /// Returns the function of `ctypes` that `function` names, as `ctypes.byref` or an
    /// imported `byref`, when no local name hides it.
    fn ctypes_function_of(&self, function: &Expr) -> Option<CtypesFunction> {
        let found = match function {
            Expr::Name { id, .. } if !self.locals.contains_key(id) => {
                self.program.module_functions.get(id).copied()
            }
            Expr::Attribute {
                value, attribute, ..
            } if self.module_of(value).as_deref() == Some("ctypes") => match self
                .program
                .builtin_lookup("ctypes", &attribute.name, attribute.span)
            {
                Ok(BuiltinName::Function(function)) => Some(function),
                _ => None,
            },
            _ => None,
        };
        match found {
            Some(ModuleFunction::Ctypes(function)) => Some(function),
            _ => None,
        }
    }

    /// Checks an argument of a C function: `ctypes.byref(object)` is the object passed by
    /// reference, and any other argument is a value.
    pub(super) fn foreign_argument(&mut self, argument: &Expr) -> TypedExpr {
        let Expr::Call {
            function,
            arguments,
            keywords,
            span,
        } = argument
        else {
            return self.expression(argument);
        };
        if self.ctypes_function_of(function) != Some(CtypesFunction::Reference) {
            return self.expression(argument);
        }
        let ([object], []) = (arguments.as_slice(), keywords.as_slice()) else {
            return self.ctypes_arity("ctypes.byref", "1", arguments, keywords, *span);
        };
        let object = self.expression(object);
        match &object.ty {
            Type::CData(CData::Scalar(_) | CData::Array(..) | CData::Structure(_)) => {
                let ty = object.ty.clone();
                Self::typed(
                    TypedExprKind::CData(CDataExpr::Reference(Box::new(object))),
                    ty,
                    *span,
                )
            }
            Type::CData(CData::Pointer(_)) => {
                self.error(planned(*span, "`byref` of a pointer"), *span)
            }
            Type::Error => object,
            other => {
                let message = format!(
                    "`ctypes.byref` takes a `ctypes` object, and {} is given",
                    other.with_article()
                );
                self.error(mismatch(404, object.span, message), *span)
            }
        }
    }

    /// Returns the error for a call of a function of `ctypes` with other than `wanted`
    /// positional arguments, after checking them.
    fn ctypes_arity(
        &mut self,
        name: &str,
        wanted: &str,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        for argument in arguments {
            self.expression(argument);
        }
        let given = arguments.len() + keywords.len();
        let message = format!(
            "`{name}` takes {wanted} positional argument{}, and {given} {} given",
            if wanted == "1" { "" } else { "s" },
            if given == 1 { "is" } else { "are" }
        );
        self.error(mismatch(404, span, message), span)
    }

    /// Checks a call of `ctypes.POINTER`, `ctypes.pointer`, `ctypes.byref` or
    /// `ctypes.sizeof`.
    pub(super) fn ctypes_call(
        &mut self,
        function: CtypesFunction,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let name = function.name();
        let ([argument], []) = (arguments, keywords) else {
            return self.ctypes_arity(&format!("ctypes.{name}"), "1", arguments, keywords, span);
        };
        let argument = self.expression_or_class(argument);
        if argument.ty.has_error() {
            return argument;
        }
        match function {
            CtypesFunction::PointerType => match argument.ty {
                Type::CDataClass(target) if target.is_item() => {
                    Self::cdata_class(CData::Pointer(Box::new(target)), span)
                }
                Type::CDataClass(_) => self.error(pointer_of_pointer(span), span),
                other => {
                    let message = format!(
                        "`ctypes.POINTER` takes a `ctypes` class, and {} is given",
                        other.with_article()
                    );
                    self.error(mismatch(404, span, message), span)
                }
            },
            CtypesFunction::Pointer => {
                let argument = self.refuse_class(argument);
                self.pointer_to(argument, span)
            }
            CtypesFunction::Reference => {
                self.refuse_class(argument);
                self.error(
                    planned(span, "`ctypes.byref` other than as a C function's argument"),
                    span,
                )
            }
            CtypesFunction::Size => match argument.ty {
                Type::CDataClass(class) => Self::typed(
                    TypedExprKind::Constant(TypedConstant::Int(class.size().to_string())),
                    Type::Int,
                    span,
                ),
                Type::CData(_) => Self::typed(
                    TypedExprKind::CData(CDataExpr::Size(Box::new(argument))),
                    Type::Int,
                    span,
                ),
                other => {
                    let message = format!(
                        "`ctypes.sizeof` takes a `ctypes` class or object, and {} has no size",
                        other.with_article()
                    );
                    self.error(mismatch(404, span, message), span)
                }
            },
        }
    }

    /// Returns a new pointer to `target`, as `ctypes.pointer(target)` makes it.
    fn pointer_to(&mut self, target: TypedExpr, span: Span) -> TypedExpr {
        match &target.ty {
            Type::CData(class) if class.is_item() => {
                let ty = Type::CData(CData::Pointer(Box::new(class.clone())));
                Self::typed(
                    TypedExprKind::CData(CDataExpr::Pointer(Box::new(target))),
                    ty,
                    span,
                )
            }
            Type::CData(_) => self.error(pointer_of_pointer(span), span),
            Type::Error => target,
            other => {
                let message = format!(
                    "`ctypes.pointer` takes a `ctypes` object, and {} is given",
                    other.with_article()
                );
                self.error(mismatch(404, target.span, message), span)
            }
        }
    }

    /// Checks `class(arguments)`: a scalar takes its value, or none for zero; an array its
    /// items, the rest zero; a pointer class the object it points to, or none for null; a
    /// structure its fields' values.
    pub(super) fn cdata_new(
        &mut self,
        class: &CData,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let name = class.class_name();
        if let CData::Structure(structure) = class {
            return self.new_structure(structure, arguments, keywords, span);
        }
        if let Some(keyword) = keywords.first() {
            for argument in arguments {
                self.expression(argument);
            }
            let message = format!("`{name}` takes positional arguments only");
            return self.error(mismatch(404, keyword.span, message), span);
        }
        let ty = Type::CData(class.clone());
        let (stored, what) = match class {
            CData::Pointer(target) => return self.new_pointer((target, &name), arguments, span),
            CData::Scalar(_) if arguments.len() > 1 => {
                return self.ctypes_arity(&name, "0 or 1", arguments, keywords, span);
            }
            CData::Array(item, _) => (item.as_ref(), format!("an item of `{name}` is")),
            _ => (class, format!("the value of `{name}` is")),
        };
        // An array given more items than it holds raises `IndexError` when it is made, as
        // CPython's does.
        let values: Vec<TypedExpr> = arguments
            .iter()
            .map(|argument| {
                let value = self.expression(argument);
                self.check_value(stored, &value.ty, (404, value.span), &what);
                value
            })
            .collect();
        Self::typed(TypedExprKind::CData(CDataExpr::New(values)), ty, span)
    }

    /// Checks `POINTER(T)()`, a null pointer, or `POINTER(T)(target)`, a pointer to an object
    /// of `T`.
    fn new_pointer(
        &mut self,
        (target, name): (&CData, &str),
        arguments: &[Expr],
        span: Span,
    ) -> TypedExpr {
        let ty = Type::CData(CData::Pointer(Box::new(target.clone())));
        match arguments {
            [] => Self::typed(TypedExprKind::CData(CDataExpr::New(Vec::new())), ty, span),
            [argument] => {
                let argument = self.expression(argument);
                if !argument.ty.has_error() && argument.ty != Type::CData(target.clone()) {
                    let message = format!(
                        "`{name}` takes a `{}`, and {} is given",
                        target.class_name(),
                        argument.ty.with_article()
                    );
                    return self.error(mismatch(404, argument.span, message), span);
                }
                self.pointer_to(argument, span)
            }
            _ => self.ctypes_arity(name, "0 or 1", arguments, &[], span),
        }
    }

    /// Checks `S(arguments)` of a structure class: positional arguments give the fields in
    /// order, and keywords give fields by name, each once.
    fn new_structure(
        &mut self,
        structure: &StructureClass,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let name = &structure.name;
        let mut given: Vec<(usize, TypedExpr)> = Vec::new();
        for (position, argument) in arguments.iter().enumerate() {
            let value = self.expression(argument);
            let Some(field) = structure.fields.get(position) else {
                let message = format!(
                    "`{name}` has {} field{}, and {} positional arguments are given",
                    structure.fields.len(),
                    if structure.fields.len() == 1 { "" } else { "s" },
                    arguments.len()
                );
                self.diagnostics.push(mismatch(404, value.span, message));
                continue;
            };
            let what = format!("the field `{}` of `{name}` is", field.name);
            self.check_value(&field.class, &value.ty, (404, value.span), &what);
            given.push((position, value));
        }
        for keyword in keywords {
            let value = self.expression(&keyword.value);
            // The subset check refuses `**` before a call is checked, so a keyword has a name.
            let field_name = keyword
                .name
                .as_ref()
                .map_or("**", |field_name| field_name.name.as_str());
            let Some((position, field)) = structure.field(field_name) else {
                let message = format!("`{name}` has no field `{field_name}`");
                self.diagnostics.push(mismatch(404, keyword.span, message));
                continue;
            };
            if given.iter().any(|(each, _)| *each == position) {
                let message = format!("the field `{}` of `{name}` is given twice", field.name);
                self.diagnostics.push(mismatch(404, keyword.span, message));
                continue;
            }
            let what = format!("the field `{}` of `{name}` is", field.name);
            self.check_value(&field.class, &value.ty, (404, value.span), &what);
            given.push((position, value));
        }
        let ty = Type::CData(CData::Structure(std::sync::Arc::new(structure.clone())));
        Self::typed(
            TypedExprKind::CData(CDataExpr::NewStructure(given)),
            ty,
            span,
        )
    }

    /// Reports a value of type `ty` that a place of class `class` does not take, with `code`
    /// at `span`: a scalar takes what `ctypes` converts, and a `c_bool` any value with a
    /// truth; a structure or an array an object of its class, whose bytes are copied; a
    /// pointer one of its class, or `None`.
    fn check_value(&mut self, class: &CData, ty: &Type, (code, span): (u16, Span), what: &str) {
        if ty.has_error() {
            return;
        }
        let takes = match class {
            CData::Scalar(CType::Bool) => {
                if !ty.is_truth_testable() {
                    let message =
                        format!("{what} {}, whose truth cannot be tested", ty.with_article());
                    self.diagnostics.push(
                        Diagnostic::new(DiagnosticCode::error(304), span, message)
                            .with_rule(RuleId::EXP_10),
                    );
                }
                return;
            }
            CData::Scalar(scalar) => {
                if scalar.stores(ty) {
                    return;
                }
                scalar.stored_types().to_owned()
            }
            CData::Pointer(_) => {
                if *ty == Type::CData(class.clone()) || *ty == Type::NoneType {
                    return;
                }
                format!("a `{}` or `None`", class.class_name())
            }
            CData::Array(..) | CData::Structure(_) => {
                if *ty == Type::CData(class.clone()) {
                    return;
                }
                format!("a `{}`", class.class_name())
            }
        };
        let message = format!("{what} {}, where it takes {takes}", ty.with_article());
        self.diagnostics.push(mismatch(code, span, message));
    }

    /// Checks `class * length` or `length * class`: the length is an integer literal, so the
    /// array's class is known when the program compiles.
    pub(super) fn cdata_array_class(
        &mut self,
        class: CData,
        length: TypedExpr,
        span: Span,
    ) -> TypedExpr {
        if length.ty.has_error() {
            return length;
        }
        if !class.is_item() {
            return self.error(planned(span, "an array of pointers or arrays"), span);
        }
        let TypedExprKind::Constant(TypedConstant::Int(digits)) = &length.kind else {
            let length = self.refuse_class(length);
            if length.ty.has_error() {
                return length;
            }
            return self.error(
                planned(span, "an array whose length is not an integer literal"),
                span,
            );
        };
        let Ok(count) = digits.parse::<u32>() else {
            let message = format!("an array of {digits} items is too large for TyPy");
            return self.error(mismatch(404, length.span, message), span);
        };
        Self::cdata_class(CData::Array(Box::new(class), count), span)
    }

    /// Checks `object.value`, `pointer.contents` and `structure.field`, where `object` is of
    /// class `class`.
    pub(super) fn cdata_attribute(
        &mut self,
        (object, class): (TypedExpr, CData),
        attribute: &Identifier,
        span: Span,
    ) -> TypedExpr {
        match (attribute.name.as_str(), &class) {
            ("value", CData::Scalar(scalar)) => Self::typed(
                TypedExprKind::CData(CDataExpr::Value(Box::new(object))),
                scalar.result_type(),
                span,
            ),
            ("contents", CData::Pointer(target)) => Self::typed(
                TypedExprKind::CData(CDataExpr::Contents(Box::new(object))),
                Type::CData(*target.clone()),
                span,
            ),
            (name, CData::Structure(structure))
                if let Some((field, found)) = structure.field(name) =>
            {
                Self::typed(
                    TypedExprKind::CData(CDataExpr::Field {
                        object: Box::new(object),
                        field,
                    }),
                    found.class.result_type(),
                    span,
                )
            }
            (name, _) => {
                let error = no_attribute(&class, name, attribute.span);
                self.error(error, span)
            }
        }
    }

    /// Checks `object[index]` of a pointer or an array of class `class`: the value of a
    /// scalar item, or an object over a structure item.
    pub(super) fn cdata_item(
        &mut self,
        (object, class): (TypedExpr, &CData),
        index: &Expr,
        span: Span,
    ) -> TypedExpr {
        if matches!(index, Expr::Slice { .. }) {
            return self.error(
                planned(span, "a slice of a `ctypes` array or pointer"),
                span,
            );
        }
        let index = self.expression(index);
        let Some(item) = self.indexed_item(class, &index, span) else {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        };
        Self::typed(
            TypedExprKind::CData(CDataExpr::Item {
                object: Box::new(object),
                index: Box::new(index),
            }),
            item.result_type(),
            span,
        )
    }

    /// Returns the class of the items `object[index]` reads of an object of class `class`,
    /// after reporting a class that has no items or an index that is not an `int`.
    fn indexed_item<'class>(
        &mut self,
        class: &'class CData,
        index: &TypedExpr,
        span: Span,
    ) -> Option<&'class CData> {
        self.expect(index, Type::Int, "an index");
        let item = class.item();
        if item.is_none() {
            let message = format!("a `{}` cannot be indexed", class.class_name());
            self.diagnostics.push(mismatch(401, span, message));
        }
        item
    }

    /// Checks `object[index] = value` of a pointer or an array, where `value` is of type `ty`.
    pub(super) fn cdata_item_target(
        &mut self,
        (object, class): (TypedExpr, &CData),
        index: TypedExpr,
        ty: &Type,
        span: Span,
    ) -> Option<AssignTarget> {
        let item = self.indexed_item(class, &index, span)?;
        let what = format!("an item of `{}` is assigned", object.ty);
        self.check_value(item, ty, (402, span), &what);
        Some(AssignTarget::CData {
            object: Box::new(object),
            place: CDataPlace::Item(Box::new(index)),
        })
    }

    /// Checks `object.value = value` and `structure.field = value`, where `value` is of type
    /// `ty`; a pointer's `contents` is not assigned until 1.0.
    pub(super) fn cdata_attribute_target(
        &mut self,
        (object, class): (TypedExpr, CData),
        attribute: &Identifier,
        ty: &Type,
        span: Span,
    ) -> Option<AssignTarget> {
        let place = match (attribute.name.as_str(), &class) {
            ("value", CData::Scalar(_)) => {
                let what = format!("the value of `{}` is assigned", class.class_name());
                self.check_value(&class, ty, (402, span), &what);
                CDataPlace::Value
            }
            (name, CData::Structure(structure))
                if let Some((field, found)) = structure.field(name) =>
            {
                let what = format!("the field `{name}` of `{}` is assigned", structure.name);
                self.check_value(&found.class, ty, (402, span), &what);
                CDataPlace::Field(field)
            }
            ("contents", CData::Pointer(_)) => {
                self.diagnostics
                    .push(planned(span, "assigning a pointer's `contents`"));
                return None;
            }
            (name, _) => {
                self.diagnostics
                    .push(no_attribute(&class, name, attribute.span));
                return None;
            }
        };
        Some(AssignTarget::CData {
            object: Box::new(object),
            place,
        })
    }
}

/// Returns the error for an attribute a `ctypes` object does not have.
fn no_attribute(class: &CData, name: &str, span: Span) -> Diagnostic {
    let message = match class {
        CData::Structure(structure) => format!("`{}` has no field `{name}`", structure.name),
        other => format!(
            "a `{}` has no attribute `{name}` in TyPy",
            other.class_name()
        ),
    };
    mismatch(412, span, message)
}
