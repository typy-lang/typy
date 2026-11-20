//! Checks calls of the methods of built-in types: binds each argument to its parameter, by
//! position or by name, and checks its type (LIB-03).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Identifier, Keyword};

use super::BodyChecker;
use crate::method::{Method, MethodSignature, Parameter, ParameterKind, ParameterType};
use crate::typed::{Argument, FileAttribute, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks a call through `super()`, a copy of a generic class named with its type
    /// arguments, or a module object, which names a method of a base, of the copy, or a
    /// function of the module rather than a method of a value.
    fn call_through(
        &mut self,
        receiver: &Expr,
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        if is_super(receiver) {
            return Some(self.super_call(receiver, name, arguments, keywords, span));
        }
        if let Some(call) = self.foreign_call(receiver, name, arguments, keywords, span) {
            return Some(call);
        }
        if let Some(copy) = self.applied_class(receiver) {
            return Some(self.class_method_call(copy, name, arguments, keywords, span));
        }
        let module = self.module_of(receiver)?;
        let call = self.module_call(&module, name, arguments, keywords, span);
        Some(self.through_lazy_module(receiver, call))
    }

    /// Returns the copy `Class[arguments]` names, when the receiver names a generic class of
    /// the program, by its name or through its module, with its type arguments (ADR-0028).
    fn applied_class(&mut self, receiver: &Expr) -> Option<crate::typed::ClassId> {
        let Expr::Subscript { value, .. } = receiver else {
            return None;
        };
        let class = match value.as_ref() {
            Expr::Name { id, .. } if !self.locals.contains_key(id) => {
                self.program.class_ids.get(id).copied()?
            }
            Expr::Attribute {
                value: module,
                attribute,
                ..
            } => match module.as_ref() {
                Expr::Name { id, .. } => {
                    *self.program.module_classes.get(id)?.get(&attribute.name)?
                }
                _ => return None,
            },
            _ => return None,
        };
        if !self
            .program
            .class_copies
            .borrow()
            .roots
            .contains_key(&class)
        {
            return None;
        }
        Some(self.annotation(receiver).class_id().unwrap_or(class))
    }

    /// Checks `receiver.name(arguments)`, a method of a built-in type (LIB-03).
    /// Returns the position of the `key` of `list.sort`, unless it is `None`, which is no
    /// key (FUN-05).
    fn sort_key(method: Method, keywords: &[Keyword]) -> Option<usize> {
        let position = Self::keyword_position(keywords, "key")?;
        let none = matches!(
            keywords[position].value,
            Expr::Constant {
                value: typy_source::syntax::Constant::None,
                ..
            }
        );
        (method == Method::ListSort && !none).then_some(position)
    }

    /// Returns the error for `setdefault` without a default, which stores `None`, when the
    /// dict's value type is not optional (DAT-09).
    fn setdefault_without_default(
        &mut self,
        method: Method,
        receiver: &TypedExpr,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        let Type::Dict(_, value) = &receiver.ty else {
            return None;
        };
        if method != Method::DictSetdefault
            || arguments.len() != 1
            || !keywords.is_empty()
            || matches!(value.as_ref(), Type::Optional(_))
        {
            return None;
        }
        let value = value.as_ref().clone();
        for argument in arguments {
            self.expression(argument);
        }
        Some(
            self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "`setdefault` without a default stores None, which a value of type {value} \
                     cannot be"
                    ),
                )
                .with_rule(RuleId::DAT_09)
                .with_help(
                    "give a default of the value type, such as `counts.setdefault(word, 0)`",
                ),
                span,
            ),
        )
    }

    pub(super) fn method_call(
        &mut self,
        receiver: &Expr,
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        if let Some(call) = self.call_through(receiver, name, arguments, keywords, span) {
            return call;
        }
        let receiver = {
            let receiver = self.expression(receiver);
            self.through_union_base(receiver)
        };
        if receiver.ty.has_error() {
            return Self::invalid_call(span);
        }
        if name.name == "fromkeys"
            && let Type::TypeObject(Some(of)) = &receiver.ty
            && **of == crate::types::TypeObjectOf::Dict
        {
            return self.call_fromkeys(arguments, keywords, None, span);
        }
        if let Type::TypeObject(Some(of)) = &receiver.ty
            && let Some(class) = Self::program_class(of)
        {
            return self.class_method_call(class, name, arguments, keywords, span);
        }
        if let Some(class) = receiver.ty.class_id() {
            return self.instance_method_call(receiver, class, name, arguments, keywords, span);
        }
        let Some(method) = Method::of(&receiver.ty, &name.name) else {
            return self.unknown_method(&receiver.ty, name, span);
        };
        if let Some(position) = Self::sort_key(method, keywords) {
            return self.keyed_sort(receiver, arguments, keywords, position, span);
        }
        // A program that may close a text file may close `sys.stdout`, so its `print` checks
        // that standard output is open (LIB-14).
        if method == Method::TextClose && receiver.ty == Type::TextFile {
            self.program.closes_text_files = true;
        }
        if let Some(refused) =
            self.setdefault_without_default(method, &receiver, arguments, keywords, span)
        {
            return refused;
        }
        let signature = method.signature(&receiver.ty);
        let Some(bound) = self.bind(method, &signature, arguments, keywords, span) else {
            return Self::invalid_call(span);
        };
        let text = if receiver.ty == Type::Bytes || receiver.ty.binary_kind().is_some() {
            Type::Bytes
        } else {
            Type::Str
        };
        let arguments: Vec<Argument> = bound
            .into_iter()
            .map(|(parameter, expression)| Argument {
                keyword: None,
                parameter,
                value: self.method_argument(
                    method,
                    &signature.parameters[parameter],
                    expression,
                    &text,
                ),
            })
            .collect();
        // `get` without a default gives `None` for a missing key (TYP-06). For an optional value
        // type that is `get(key, None)`, since `None` is already one of its values.
        let mut arguments = arguments;
        let result = match (method, arguments.len(), &signature.result) {
            (Method::DictGet, 1, Type::Optional(_)) => {
                let none = Self::typed(
                    TypedExprKind::Constant(TypedConstant::None),
                    Type::NoneType,
                    span,
                );
                arguments.push(Argument {
                    keyword: None,
                    parameter: 1,
                    value: Self::converted(none, Some(&signature.result)),
                });
                signature.result
            }
            (Method::DictGet, 1, _) => Type::optional(signature.result),
            _ => signature.result,
        };
        self.record_runtime_raises(method, &receiver.ty);
        Self::typed(
            TypedExprKind::Method {
                receiver: Box::new(receiver),
                method,
                arguments,
            },
            result,
            span,
        )
    }

    /// Records what a method may make the runtime raise: a `KeyError` that holds a missing
    /// key, or the Unicode error of `encode` or `decode` (ADR-0034).
    fn record_runtime_raises(&mut self, method: Method, receiver: &Type) {
        match (method, receiver) {
            (Method::DictPop, Type::Dict(key, _)) | (Method::SetRemove, Type::Set(key)) => {
                self.program.record_key(key);
            }
            (Method::StrEncode, _) => self.program.record_codec(true, false),
            (Method::BytesDecode, _) => self.program.record_codec(false, true),
            _ => {}
        }
    }

    /// Checks the value `send` passes against `sends`, the type the generator receives, and
    /// holds it as one, which the generator reads it as (FUN-07).
    fn sent_value(&mut self, value: TypedExpr, sends: &Type) -> TypedExpr {
        if value.ty == Type::NoneType && !value.ty.is_assignable_to(sends) {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    value.span,
                    format!(
                        "`value` of `send` must be {}, and this is None",
                        sends.with_article()
                    ),
                )
                .with_rule(RuleId::TYP_01)
                .with_help(
                    "start the generator with `next(generator)`; `send(None)` starts one that \
                     receives `None`, as `Generator[Y, S | None, R]` does",
                ),
            );
            return value;
        }
        self.expect(&value, sends.clone(), "`value` of `send`");
        Self::converted(value, Some(sends))
    }

    pub(super) fn invalid_call(span: Span) -> TypedExpr {
        Self::typed(
            TypedExprKind::Constant(TypedConstant::None),
            Type::Error,
            span,
        )
    }

    /// Binds the arguments of a call to the parameters, in source order: the positional ones in
    /// turn, then each keyword by its name. Returns `None` after reporting a call that does not
    /// fit the signature.
    fn bind<'call>(
        &mut self,
        method: Method,
        signature: &MethodSignature,
        arguments: &'call [Expr],
        keywords: &'call [Keyword],
        span: Span,
    ) -> Option<Vec<(usize, &'call Expr)>> {
        let parameters = &signature.parameters;
        let rest = parameters
            .iter()
            .position(|parameter| parameter.kind == ParameterKind::Rest);
        let positional = parameters
            .iter()
            .filter(|parameter| parameter.kind != ParameterKind::Keyword)
            .count();
        let required = parameters
            .iter()
            .filter(|parameter| parameter.required)
            .count();
        if rest.is_none() && arguments.len() > positional {
            self.arity_error(method, positional, required, arguments.len(), span);
            return None;
        }
        // Every argument from the rest parameter on binds to it.
        let mut bound: Vec<(usize, &Expr)> = arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| (rest.map_or(index, |rest| index.min(rest)), argument))
            .collect();
        for keyword in keywords {
            let name = keyword.name.as_ref().map(|name| name.name.as_str());
            let index = parameters.iter().position(|parameter| {
                Some(parameter.name) == name
                    && matches!(
                        parameter.kind,
                        ParameterKind::Either | ParameterKind::Keyword
                    )
            });
            let message = match (name, index) {
                (Some(_), Some(index)) if bound.iter().all(|(given, _)| *given != index) => {
                    bound.push((index, &keyword.value));
                    continue;
                }
                (Some(name), Some(_)) => format!("`{name}` is given twice"),
                // `key=None` is no key; `keyed_sort` takes any other (FUN-05).
                (Some("key"), None) if method == Method::ListSort => continue,
                (Some(name), None) if parameters.iter().any(|parameter| parameter.name == name) => {
                    format!("`{name}` of `{}` is given by position only", method.name())
                }
                (Some(name), None) => {
                    format!("`{}` has no parameter named `{name}`", method.name())
                }
                (None, _) => format!("`{}` takes no `**` arguments", method.name()),
            };
            self.error(
                Diagnostic::new(DiagnosticCode::error(404), keyword.span, message)
                    .with_rule(RuleId::LIB_03),
                span,
            );
            return None;
        }
        let missing = parameters.iter().enumerate().any(|(index, parameter)| {
            parameter.required && bound.iter().all(|(given, _)| *given != index)
        });
        if missing {
            self.arity_error(method, positional, required, bound.len(), span);
            return None;
        }
        Some(bound)
    }

    fn arity_error(
        &mut self,
        method: Method,
        positional: usize,
        required: usize,
        given: usize,
        span: Span,
    ) {
        let expected = if positional == 0 && required == 0 && method == Method::ListSort {
            "no positional arguments".to_owned()
        } else if required == positional {
            counted(positional, "argument")
        } else if required == 0 {
            format!("at most {}", counted(positional, "argument"))
        } else {
            format!("{required} to {positional} arguments")
        };
        let verb = if given == 1 { "is" } else { "are" };
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                span,
                format!(
                    "`{}` takes {expected}, and {given} {verb} given",
                    method.name()
                ),
            )
            .with_rule(RuleId::LIB_03),
        );
    }

    /// Reports a method a built-in type does not have; a text file's attribute is not a
    /// method (LIB-14).
    fn unknown_method(&mut self, receiver: &Type, name: &Identifier, span: Span) -> TypedExpr {
        if receiver.is_file() {
            let Some((_, ty)) = FileAttribute::named(&name.name) else {
                return self.missing_file_name(receiver, name, span);
            };
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    name.span,
                    format!(
                        "`{}` of a text file is {} and cannot be called",
                        name.name,
                        ty.with_article()
                    ),
                )
                .with_rule(RuleId::EXP_02),
                span,
            );
        }
        let mut diagnostic = Diagnostic::new(
            DiagnosticCode::error(412),
            name.span,
            format!(
                "{} has no method `{}` in TyPy",
                receiver.with_article(),
                name.name
            ),
        )
        .with_rule(RuleId::LIB_03);
        if matches!(receiver, Type::Iterator(_) | Type::Iterable(_))
            && matches!(name.name.as_str(), "send" | "throw" | "close")
        {
            diagnostic = diagnostic.with_rule(RuleId::FUN_07).with_help(
                "only a `Generator[Y, S, R]` has `send`, `throw` and `close`; annotate the \
                 generator function with it",
            );
        }
        self.error(diagnostic, span)
    }

    /// Checks one argument against what its parameter accepts.
    fn method_argument(
        &mut self,
        method: Method,
        parameter: &Parameter,
        argument: &Expr,
        text: &Type,
    ) -> TypedExpr {
        let what = format!("`{}` of `{}`", parameter.name, method.name());
        match &parameter.ty {
            ParameterType::Exact(ty) => {
                let value = self.expression_expecting(argument, Some(ty));
                if method == Method::GeneratorSend {
                    return self.sent_value(value, ty);
                }
                self.expect(&value, ty.clone(), &what);
                value
            }
            ParameterType::Exception => self.exception_operand(argument, "`throw`"),
            ParameterType::StrOrNone | ParameterType::IntOrNone => {
                let value = self.expression(argument);
                if !matches!(
                    argument,
                    Expr::Constant {
                        value: Constant::None,
                        ..
                    }
                ) {
                    let wanted = if parameter.ty == ParameterType::IntOrNone {
                        Type::Int
                    } else {
                        text.clone()
                    };
                    self.expect(&value, wanted, &what);
                }
                value
            }
            ParameterType::Affixes => {
                let wanted = Type::TupleOf(Box::new(text.clone()));
                let value = self.expression_expecting(argument, Some(&wanted));
                if value.ty != wanted {
                    self.expect(&value, text.clone(), &what);
                }
                value
            }
            ParameterType::Codec | ParameterType::Handler { .. } => {
                self.codec_argument(&parameter.ty, argument, &what)
            }
            ParameterType::Items(item) => self.items_argument(argument, item, &what),
            ParameterType::Pairs(key, value) => self.pairs_argument(argument, key, value, &what),
            ParameterType::Strings => {
                let value = self.iterable_argument(argument, crate::builtin::Builtin::List);
                if let Some(item) = value.ty.sequence_item()
                    && item != text
                    && !item.has_error()
                {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(404),
                            value.span,
                            if method == Method::TextWritelines {
                                format!(
                                    "`writelines` writes items of type {text}, and these items \
                                     are {item}"
                                )
                            } else {
                                format!(
                                    "`join` joins items of type {text}, and these items are {item}"
                                )
                            },
                        )
                        .with_rule(RuleId::LIB_03),
                    );
                }
                value
            }
        }
    }
}

impl BodyChecker<'_> {
    /// Checks the name of a codec or an error handler, which is a string literal so it can be
    /// resolved when compiling, and returns its number as an `int` constant.
    pub(super) fn codec_argument(
        &mut self,
        kind: &ParameterType,
        argument: &Expr,
        what: &str,
    ) -> TypedExpr {
        let span = argument.span();
        let Expr::Constant {
            value: Constant::Str(name),
            ..
        } = argument
        else {
            self.expression(argument);
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!("{what} is a string literal, so TyPy can check it when compiling"),
                )
                .with_rule(RuleId::LIB_03),
                span,
            );
        };
        let (number, provided) = match kind {
            ParameterType::Handler { decoding } => (
                crate::codec::handler(name, *decoding),
                if *decoding {
                    "the error handlers `strict`, `ignore`, `replace`, `backslashreplace` and `surrogateescape`"
                } else {
                    "the error handlers `strict`, `ignore`, `replace`, `backslashreplace`, `xmlcharrefreplace` and `surrogateescape`"
                },
            ),
            _ => (
                crate::codec::codec(name),
                "the codecs `utf-8`, `ascii` and `latin-1`",
            ),
        };
        let Some(number) = number else {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!("TyPy provides {provided}, and {name:?} is none of them"),
                )
                .with_rule(RuleId::LIB_03),
                span,
            );
        };
        Self::typed(
            TypedExprKind::Constant(TypedConstant::Int(number.to_string())),
            Type::Int,
            span,
        )
    }
}

/// Returns `count` with the noun, plural unless the count is one: "1 argument", "2 arguments".
fn counted(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// Returns true for `super()`, with no arguments or two given by position.
fn is_super(receiver: &Expr) -> bool {
    matches!(
        receiver,
        Expr::Call { function, arguments, keywords, .. }
            if matches!(function.as_ref(), Expr::Name { id, .. } if id == "super")
                && matches!(arguments.len(), 0 | 2)
                && keywords.is_empty()
    )
}
