//! Checks what a program does with `ctypes`: the library a declaration loads, and each call
//! of a function its `argtypes` and `restype` declare, with the arguments `ctypes` converts
//! and the objects it passes by reference (FFI-01 to FFI-03, ADR-0047, ADR-0048).

use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Keyword};

use super::BodyChecker;
use crate::check::ctypes_declarations::ModuleDeclarations;
use crate::ctypes::{CData, CType, ForeignFunction, ForeignLibrary};
use crate::typed::CDataExpr;
use crate::typed::{TypedExpr, TypedExprKind};
use crate::types::Type;

/// The `ctypes` declarations of every module, and the libraries and functions the checked
/// program uses.
#[derive(Clone, Debug, Default)]
pub(crate) struct Foreign {
    /// The declarations of each module, by its name.
    declarations: BTreeMap<String, ModuleDeclarations>,
    /// The libraries, indexed by `Type::ForeignLibrary`.
    pub(crate) libraries: Vec<ForeignLibrary>,
    /// The module and the variable each library is declared and bound in.
    sites: Vec<(String, String)>,
    /// The functions, indexed by `TypedExprKind::ForeignCall`.
    pub(crate) functions: Vec<ForeignFunction>,
    /// The `ctypes.Structure` subclasses, by their number (ADR-0049).
    structures: Vec<std::sync::Arc<crate::ctypes::StructureClass>>,
}

impl Foreign {
    /// Sets the declarations each module makes, and the program's structures.
    pub(crate) fn set_declarations(
        &mut self,
        declarations: BTreeMap<String, ModuleDeclarations>,
        structures: Vec<std::sync::Arc<crate::ctypes::StructureClass>>,
    ) {
        self.declarations = declarations;
        self.structures = structures;
    }

    /// Returns the structure class a hidden name a structure class statement became stands
    /// for.
    pub(crate) fn structure_named(&self, name: &str) -> Option<CData> {
        let id = crate::check::ctypes_declarations::hidden_id(name)?;
        let class = self.structures.get(id as usize)?;
        Some(CData::Structure(std::sync::Arc::clone(class)))
    }

    /// Returns the index of the library that the call at `call` in `module` declares,
    /// registering it the first time.
    fn library(&mut self, module: &str, call: Span) -> Option<u32> {
        let declared = self.declarations.get(module)?.library_at(call)?.clone();
        let site = (module.to_owned(), declared.variable.clone());
        let index = self
            .sites
            .iter()
            .position(|each| *each == site)
            .unwrap_or_else(|| {
                self.libraries.push(ForeignLibrary {
                    name: declared.name,
                });
                self.sites.push(site);
                self.sites.len() - 1
            });
        u32::try_from(index).ok()
    }

    /// Returns the library bound to the module variable `variable` of `module`.
    fn library_of(&self, module: &str, variable: &str) -> Option<u32> {
        let index = self
            .sites
            .iter()
            .position(|(each_module, each)| each_module == module && each == variable)?;
        u32::try_from(index).ok()
    }

    /// Returns the index of `function` of `library`, registering it the first time.
    fn function(
        &mut self,
        library: u32,
        name: &str,
        parameters: &[CData],
        result: Option<CData>,
    ) -> u32 {
        let index = self
            .functions
            .iter()
            .position(|each| each.library == library && each.name == name)
            .unwrap_or_else(|| {
                self.functions.push(ForeignFunction {
                    library,
                    name: name.to_owned(),
                    parameters: parameters.to_vec(),
                    result,
                });
                self.functions.len() - 1
            });
        u32::try_from(index).unwrap_or(u32::MAX)
    }
}

fn refused(span: Span, message: String, rule: RuleId, help: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::error(430), span, message)
        .with_rule(rule)
        .with_help(help.to_owned())
}

impl BodyChecker<'_> {
    /// Returns the library the call at `span` declares, when it is a module's declaration of
    /// one: its value holds nothing when the program runs.
    pub(super) fn foreign_library(&mut self, span: Span) -> Option<TypedExpr> {
        let module = self.program.module.clone();
        let library = self.program.foreign.library(&module, span)?;
        Some(Self::typed(
            TypedExprKind::ForeignLibrary(library),
            Type::ForeignLibrary(library),
            span,
        ))
    }

    /// Returns the library a call's receiver names: a module variable a declaration binds,
    /// which no local name hides.
    fn receiver_library(&self, receiver: &Expr) -> Option<(u32, String)> {
        let Expr::Name { id, .. } = receiver else {
            return None;
        };
        let hidden =
            self.in_function() && self.locals.contains_key(id) && !self.global_names.contains(id);
        if hidden {
            return None;
        }
        let library = self.program.foreign.library_of(&self.program.module, id)?;
        Some((library, id.clone()))
    }

    /// Checks `library.function(arguments)` when the receiver names a library: a call of the
    /// C function its declarations describe.
    pub(super) fn foreign_call(
        &mut self,
        receiver: &Expr,
        function: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        let (library, variable) = self.receiver_library(receiver)?;
        let declared = self
            .program
            .foreign
            .declarations
            .get(&self.program.module)
            .and_then(|declarations| declarations.function(&variable, &function.name))
            .cloned();
        let typed_arguments: Vec<TypedExpr> = arguments
            .iter()
            .map(|argument| self.foreign_argument(argument))
            .collect();
        let name = format!("{variable}.{}", function.name);
        let Some((parameters, result, declared_at)) = declared.and_then(|declared| {
            Some((
                declared.parameters?,
                declared.result?.c_type(),
                declared.declared,
            ))
        }) else {
            return Some(self.error(
                refused(
                    span,
                    format!("`{name}` is called without both its `argtypes` and its `restype`"),
                    RuleId::FFI_02,
                    "assign `argtypes` and `restype` at module level before the first call",
                ),
                span,
            ));
        };
        if !self.in_function() && span.start() < declared_at {
            return Some(self.error(
                refused(
                    span,
                    format!("`{name}` is called before its `argtypes` and `restype` are assigned"),
                    RuleId::FFI_02,
                    "move the call after the assignments",
                ),
                span,
            ));
        }
        if let Some(keyword) = keywords.first() {
            return Some(
                self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        keyword.span,
                        format!("`{name}` is a C function, which takes positional arguments only"),
                    )
                    .with_rule(RuleId::FFI_02),
                    span,
                ),
            );
        }
        if typed_arguments.len() != parameters.len() {
            let wanted = parameters.len();
            return Some(
                self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        span,
                        format!(
                            "`{name}` takes {wanted} argument{}, as its `argtypes` declare, and {} \
                         {} given",
                            if wanted == 1 { "" } else { "s" },
                            typed_arguments.len(),
                            if typed_arguments.len() == 1 {
                                "was"
                            } else {
                                "were"
                            },
                        ),
                    )
                    .with_rule(RuleId::FFI_02),
                    span,
                ),
            );
        }
        self.foreign_arguments(&name, &typed_arguments, &parameters);
        let ty = result.as_ref().map_or(Type::NoneType, CData::result_type);
        let index = self
            .program
            .foreign
            .function(library, &function.name, &parameters, result);
        Some(Self::typed(
            TypedExprKind::ForeignCall {
                function: index,
                arguments: typed_arguments,
            },
            ty,
            span,
        ))
    }

    /// Reports each argument of `name` whose type its `ctypes` type does not take; a
    /// `c_bool` argument takes any value with a truth.
    fn foreign_arguments(&mut self, name: &str, arguments: &[TypedExpr], parameters: &[CData]) {
        for (position, (argument, parameter)) in arguments.iter().zip(parameters).enumerate() {
            if argument.ty.has_error() {
                continue;
            }
            let by_reference =
                matches!(argument.kind, TypedExprKind::CData(CDataExpr::Reference(_)));
            if *parameter == CData::Scalar(CType::Bool) && !by_reference {
                self.check_truth(argument);
                continue;
            }
            if !parameter.accepts(&argument.ty, by_reference) {
                let given = if by_reference {
                    format!("`byref` of {}", argument.ty.with_article())
                } else {
                    argument.ty.with_article()
                };
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        argument.span,
                        format!(
                            "argument {} of `{name}` is {given}, where its `argtypes` take {}",
                            position + 1,
                            parameter.argument_types()
                        ),
                    )
                    .with_rule(RuleId::FFI_03),
                );
            }
        }
    }

    /// Returns the error for a library used other than to call its functions, which is
    /// planned for 0.13 (FFI-09).
    pub(super) fn library_as_value(&mut self, name: &str, span: Span) -> TypedExpr {
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!(
                    "the library `{name}` used other than to call its functions is planned for \
                     TyPy 0.13 (FFI-09)"
                ),
            )
            .with_rule(RuleId::FFI_09)
            .with_help(format!("call its functions, as `{name}.function(...)`")),
            span,
        )
    }

    /// Returns the error for a call of `ctypes.CDLL` or `ctypes.cdll.LoadLibrary` that is not
    /// a module's declaration of a library.
    pub(super) fn misplaced_library(&mut self, arguments: &[Expr], span: Span) -> TypedExpr {
        for argument in arguments {
            self.expression(argument);
        }
        self.error(
            refused(
                span,
                "a library is declared at module level and bound to a name, as \
                 `library = ctypes.CDLL(\"name\")`"
                    .to_owned(),
                RuleId::FFI_01,
                "load the library once, at module level, and call its functions through the name",
            ),
            span,
        )
    }
}
