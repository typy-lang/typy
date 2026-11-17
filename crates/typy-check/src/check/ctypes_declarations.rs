//! Reads the `ctypes` declarations of a module from its statements at module level: each
//! library a `CDLL` or `cdll.LoadLibrary` call with a string literal names, and each
//! function's `argtypes` and `restype`, whose types are `ctypes` scalars, `POINTER(T)`, or
//! names an earlier module-level assignment binds to one. The assignments to `argtypes` and
//! `restype` then leave the module, since the declarations do their work when the program is
//! compiled (FFI-01 to FFI-03, ADR-0047, ADR-0048).

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span, TextOffset};
use typy_source::syntax::{Constant, Expr, Stmt};

use crate::ctypes::{CData, CType};

/// A library a module declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DeclaredLibrary {
    /// The module variable the library is bound to.
    pub(crate) variable: String,
    /// The name the declaration gives.
    pub(crate) name: String,
    /// The call that declares it, by which the checker finds it.
    pub(crate) call: Span,
}

/// A function a module declares on one of its libraries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DeclaredFunction {
    /// The module variable of its library.
    pub(crate) variable: String,
    /// Its C symbol.
    pub(crate) function: String,
    /// The `ctypes` type of each parameter, once `argtypes` is assigned.
    pub(crate) parameters: Option<Vec<CData>>,
    /// The result `restype` declares, once it is assigned.
    pub(crate) result: Option<Restype>,
    /// Where the later of the two assignments ends: a call at module level before it is too
    /// early.
    pub(crate) declared: TextOffset,
}

/// What a function's `restype` declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Restype {
    /// `None`: the function returns nothing.
    Void,
    /// A `ctypes` type.
    Of(CData),
}

impl Restype {
    /// Returns the type of the result, or `None` for `void`.
    pub(crate) fn c_type(self) -> Option<CData> {
        match self {
            Self::Void => None,
            Self::Of(ty) => Some(ty),
        }
    }
}

/// The `ctypes` declarations of one module.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModuleDeclarations {
    pub(crate) libraries: Vec<DeclaredLibrary>,
    pub(crate) functions: Vec<DeclaredFunction>,
}

impl ModuleDeclarations {
    /// Returns the library the call at `call` declares.
    pub(crate) fn library_at(&self, call: Span) -> Option<&DeclaredLibrary> {
        self.libraries.iter().find(|library| library.call == call)
    }

    /// Returns the declaration of `function` on the library bound to `variable`.
    pub(crate) fn function(&self, variable: &str, function: &str) -> Option<&DeclaredFunction> {
        self.functions
            .iter()
            .find(|each| each.variable == variable && each.function == function)
    }
}

fn refused(span: Span, message: String, rule: RuleId, help: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::error(430), span, message)
        .with_rule(rule)
        .with_help(help.to_owned())
}

/// The names a module gives `ctypes` and its members by its imports at module level.
#[derive(Default)]
struct CtypesNames {
    /// Each name the module itself is bound to, such as `ctypes` or `C`.
    modules: Vec<String>,
    /// Each name bound to a member, with the member's own name.
    members: BTreeMap<String, String>,
}

impl CtypesNames {
    fn of(body: &[Stmt]) -> Self {
        let mut names = Self::default();
        for statement in body {
            match statement {
                Stmt::Import { names: aliases, .. } => {
                    for alias in aliases.iter().filter(|alias| alias.name.name == "ctypes") {
                        let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
                        names.modules.push(bound.name.clone());
                    }
                }
                Stmt::ImportFrom {
                    module: Some(module),
                    names: aliases,
                    level: 0,
                    ..
                } if module.name == "ctypes" => {
                    for alias in aliases {
                        let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
                        names
                            .members
                            .insert(bound.name.clone(), alias.name.name.clone());
                    }
                }
                _ => {}
            }
        }
        names
    }

    /// Returns the member of `ctypes` that `expression` names: `ctypes.name` or a name
    /// imported from it.
    fn member<'expression>(
        &'expression self,
        expression: &'expression Expr,
    ) -> Option<&'expression str> {
        match expression {
            Expr::Attribute {
                value, attribute, ..
            } => match value.as_ref() {
                Expr::Name { id, .. } if self.modules.contains(id) => Some(&attribute.name),
                _ => None,
            },
            Expr::Name { id, .. } => self.members.get(id).map(String::as_str),
            _ => None,
        }
    }

    /// Returns the `ctypes` type `expression` names in a declaration: a scalar type, a
    /// `POINTER` of a scalar type, or a name `aliases` binds; or the error to report.
    fn declared_type(
        &self,
        expression: &Expr,
        aliases: &BTreeMap<String, CData>,
    ) -> Result<CData, Box<Diagnostic>> {
        if let Expr::Name { id, .. } = expression
            && let Some(alias) = aliases.get(id)
        {
            return Ok(alias.clone());
        }
        if let Expr::Call {
            function,
            arguments,
            keywords,
            span,
        } = expression
            && self.member(function) == Some("POINTER")
        {
            let ([target], []) = (arguments.as_slice(), keywords.as_slice()) else {
                return Err(Box::new(refused(
                    *span,
                    "`POINTER` takes one `ctypes` type".to_owned(),
                    RuleId::FFI_03,
                    "name the type pointed to, as `ctypes.POINTER(ctypes.c_int)`",
                )));
            };
            return match self.declared_type(target, aliases)? {
                scalar @ CData::Scalar(_) => Ok(CData::Pointer(Box::new(scalar))),
                _ => Err(Box::new(pointer_of_pointer(*span))),
            };
        }
        self.member(expression)
            .and_then(CType::from_name)
            .map(CData::Scalar)
            .ok_or_else(|| {
                Box::new(refused(
                    expression.span(),
                    "this is not one of the `ctypes` types TyPy compiles".to_owned(),
                    RuleId::FFI_03,
                    "name a type of `ctypes`, such as `ctypes.c_int`, `ctypes.c_double`, \
                     `ctypes.c_char_p` or `ctypes.POINTER(ctypes.c_int)`",
                ))
            })
    }

    /// Returns whether `function` loads a library: `CDLL` or `cdll.LoadLibrary`.
    fn loads_library(&self, function: &Expr) -> bool {
        if self.member(function) == Some("CDLL") {
            return true;
        }
        matches!(function, Expr::Attribute { value, attribute, .. }
            if attribute.name == "LoadLibrary" && self.member(value) == Some("cdll"))
    }
}

/// Returns the error for a pointer to a pointer or an array, which is planned for 1.0.
pub(crate) fn pointer_of_pointer(span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        "a pointer to a pointer or to an array is planned for TyPy 1.0 (FFI-10)",
    )
    .with_rule(RuleId::FFI_10)
}

/// Returns the declarations of a module, and removes its assignments to `argtypes` and
/// `restype` from `body`.
pub(crate) fn read(body: &mut Vec<Stmt>, diagnostics: &mut Diagnostics) -> ModuleDeclarations {
    let names = CtypesNames::of(body);
    let mut declarations = ModuleDeclarations::default();
    if names.modules.is_empty() && names.members.is_empty() {
        return declarations;
    }
    let mut aliases = BTreeMap::new();
    body.retain(|statement| match statement {
        Stmt::Assign { targets, value, .. } => {
            // A name bound to a scalar type or a pointer type stands for it in a later
            // declaration; the assignment stays, since the name is also a class the program
            // calls.
            if let [Expr::Name { id, .. }] = targets.as_slice()
                && let Ok(ty) = names.declared_type(value, &BTreeMap::new())
            {
                aliases.insert(id.clone(), ty);
                return true;
            }
            if let [Expr::Name { id, .. }] = targets.as_slice()
                && let Expr::Call {
                    function,
                    arguments,
                    keywords,
                    span,
                } = value
                && names.loads_library(function)
            {
                library(
                    id,
                    arguments,
                    keywords,
                    *span,
                    &mut declarations,
                    diagnostics,
                );
                return true;
            }
            if let [target] = targets.as_slice()
                && let Some((variable, function, declares)) = declaration_target(target)
                && declarations
                    .libraries
                    .iter()
                    .any(|library| library.variable == variable)
            {
                let declared = declaration(
                    (&names, &aliases),
                    value,
                    declares,
                    statement.span(),
                    diagnostics,
                );
                record(
                    &mut declarations,
                    (variable, function),
                    declares,
                    declared,
                    statement.span(),
                    diagnostics,
                );
                return false;
            }
            true
        }
        _ => true,
    });
    declarations
}

/// Records the library an assignment `variable = CDLL(...)` declares.
fn library(
    variable: &str,
    arguments: &[Expr],
    keywords: &[typy_source::syntax::Keyword],
    call: Span,
    declarations: &mut ModuleDeclarations,
    diagnostics: &mut Diagnostics,
) {
    let (
        [
            Expr::Constant {
                value: Constant::Str(name),
                ..
            },
        ],
        [],
    ) = (arguments, keywords)
    else {
        diagnostics.push(refused(
            call,
            "a library is named by one string literal, which the build links".to_owned(),
            RuleId::FFI_01,
            "write the library's file name, such as `ctypes.CDLL(\"libm.so.6\")`",
        ));
        return;
    };
    declarations.libraries.push(DeclaredLibrary {
        variable: variable.to_owned(),
        name: name.clone(),
        call,
    });
}

/// Which declaration an assignment makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Declares {
    Argtypes,
    Restype,
}

/// Returns the library variable, the function and what `target` declares, for a target
/// `variable.function.argtypes` or `variable.function.restype`.
fn declaration_target(target: &Expr) -> Option<(&str, &str, Declares)> {
    let Expr::Attribute {
        value, attribute, ..
    } = target
    else {
        return None;
    };
    let declares = match attribute.name.as_str() {
        "argtypes" => Declares::Argtypes,
        "restype" => Declares::Restype,
        _ => return None,
    };
    let Expr::Attribute {
        value: library,
        attribute: function,
        ..
    } = value.as_ref()
    else {
        return None;
    };
    let Expr::Name { id, .. } = library.as_ref() else {
        return None;
    };
    Some((id, &function.name, declares))
}

/// What one assignment declares: the parameters' types, or the result's.
enum Declared {
    Parameters(Vec<CData>),
    Result(Restype),
}

/// Reads the value of an assignment to `argtypes` or `restype`, where `aliases` holds the
/// names bound to a type before it.
fn declaration(
    (names, aliases): (&CtypesNames, &BTreeMap<String, CData>),
    value: &Expr,
    declares: Declares,
    span: Span,
    diagnostics: &mut Diagnostics,
) -> Option<Declared> {
    let c_type = |expression: &Expr, diagnostics: &mut Diagnostics| match names
        .declared_type(expression, aliases)
    {
        Ok(found) => Some(found),
        Err(diagnostic) => {
            diagnostics.push(*diagnostic);
            None
        }
    };
    match declares {
        Declares::Argtypes => {
            let (Expr::List { elements, .. } | Expr::Tuple { elements, .. }) = value else {
                diagnostics.push(refused(
                    span,
                    "`argtypes` is assigned a list or tuple of `ctypes` types".to_owned(),
                    RuleId::FFI_02,
                    "write the types in the assignment, such as `[ctypes.c_double]`",
                ));
                return None;
            };
            let types: Vec<Option<CData>> = elements
                .iter()
                .map(|element| c_type(element, diagnostics))
                .collect();
            types
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .map(Declared::Parameters)
        }
        Declares::Restype => match value {
            Expr::Constant {
                value: Constant::None,
                ..
            } => Some(Declared::Result(Restype::Void)),
            other => c_type(other, diagnostics).map(|ty| Declared::Result(Restype::Of(ty))),
        },
    }
}

/// Records one declaration of `function` on `variable`, refusing one made twice.
fn record(
    declarations: &mut ModuleDeclarations,
    (variable, function): (&str, &str),
    declares: Declares,
    value: Option<Declared>,
    span: Span,
    diagnostics: &mut Diagnostics,
) {
    let position = declarations
        .functions
        .iter()
        .position(|each| each.variable == variable && each.function == function);
    if position.is_none() {
        declarations.functions.push(DeclaredFunction {
            variable: variable.to_owned(),
            function: function.to_owned(),
            parameters: None,
            result: None,
            declared: span.end(),
        });
    }
    let index = position.unwrap_or(declarations.functions.len() - 1);
    let entry = &mut declarations.functions[index];
    let again = match declares {
        Declares::Argtypes => entry.parameters.is_some(),
        Declares::Restype => entry.result.is_some(),
    };
    if again {
        let name = match declares {
            Declares::Argtypes => "argtypes",
            Declares::Restype => "restype",
        };
        diagnostics.push(refused(
            span,
            format!("`{variable}.{function}.{name}` is assigned again"),
            RuleId::FFI_02,
            "declare each function's `argtypes` and `restype` once, at module level",
        ));
        return;
    }
    entry.declared = span.end().max(entry.declared);
    match value {
        Some(Declared::Parameters(types)) => entry.parameters = Some(types),
        Some(Declared::Result(result)) => entry.result = Some(result),
        // The value was refused; the declaration counts as made, so the call is not refused
        // again for missing it.
        None => match declares {
            Declares::Argtypes => entry.parameters = Some(Vec::new()),
            Declares::Restype => entry.result = Some(Restype::Void),
        },
    }
}

#[cfg(test)]
mod tests;
