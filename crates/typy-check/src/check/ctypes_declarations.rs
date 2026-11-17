//! Reads the `ctypes` declarations of a module from its statements at module level: each
//! library a `CDLL` or `cdll.LoadLibrary` call with a string literal names, each
//! `ctypes.Structure` subclass with its `_fields_`, and each function's `argtypes` and
//! `restype`, whose types are `ctypes` scalars, `POINTER(T)`, structures, or names an earlier
//! module-level assignment binds to one. The assignments to `argtypes` and `restype` then
//! leave the module, since the declarations do their work when the program is compiled, and
//! each structure class statement becomes the binding of its name to the class (FFI-01 to
//! FFI-03, ADR-0047 to ADR-0049).

mod structure;

pub(crate) use structure::hidden_id;

use std::collections::BTreeMap;
use std::sync::Arc;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span, TextOffset};
use typy_source::ModuleName;
use typy_source::syntax::{BinaryOperator, Constant, Expr, Stmt};

use crate::ctypes::{CData, CType, StructureClass};

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

/// The names a module gives `ctypes` and its members by its imports at module level, and the
/// modules and names of the program it imports, through which it names their structures.
#[derive(Default)]
struct CtypesNames {
    /// Each name the module itself is bound to, such as `ctypes` or `C`.
    modules: Vec<String>,
    /// Each name bound to a member, with the member's own name.
    members: BTreeMap<String, String>,
    /// Each name bound to another module, `import tools`, or to a name of one,
    /// `from tools import Point`, with the module and the name.
    program: BTreeMap<String, (String, Option<String>)>,
}

impl CtypesNames {
    fn of(body: &[Stmt], package: Option<&ModuleName>) -> Self {
        let mut names = Self::default();
        for statement in body {
            match statement {
                Stmt::Import { names: aliases, .. } => {
                    for alias in aliases {
                        let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
                        if alias.name.name == "ctypes" {
                            names.modules.push(bound.name.clone());
                        } else if alias.as_name.is_some() || !alias.name.name.contains('.') {
                            names
                                .program
                                .insert(bound.name.clone(), (alias.name.name.clone(), None));
                        }
                    }
                }
                Stmt::ImportFrom {
                    module,
                    names: aliases,
                    level,
                    ..
                } => {
                    let from = module.as_ref().map(|module| module.name.as_str());
                    let Ok(base) = typy_source::absolute_name(package, *level, from) else {
                        continue;
                    };
                    for alias in aliases {
                        let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
                        if base.as_str() == "ctypes" {
                            names
                                .members
                                .insert(bound.name.clone(), alias.name.name.clone());
                        } else {
                            names.program.insert(
                                bound.name.clone(),
                                (base.as_str().to_owned(), Some(alias.name.name.clone())),
                            );
                        }
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

    /// Returns whether `function` loads a library: `CDLL` or `cdll.LoadLibrary`.
    fn loads_library(&self, function: &Expr) -> bool {
        if self.member(function) == Some("CDLL") {
            return true;
        }
        matches!(function, Expr::Attribute { value, attribute, .. }
            if attribute.name == "LoadLibrary" && self.member(value) == Some("cdll"))
    }
}

/// What a declaration's types are read with: the module's names of `ctypes` and of the
/// program, the names earlier assignments bind to a type, and the structures declared so far.
pub(super) struct Types<'scope> {
    names: &'scope CtypesNames,
    aliases: &'scope BTreeMap<String, CData>,
    structures: &'scope [Arc<StructureClass>],
}

impl Types<'_> {
    /// Returns the `ctypes` type `expression` names in a declaration: a scalar type,
    /// `POINTER` of a scalar or a structure, a structure, an array of scalars or structures
    /// when `arrays`, or a name an earlier assignment binds to one; or the error to report.
    pub(super) fn declared(
        &self,
        expression: &Expr,
        arrays: bool,
    ) -> Result<CData, Box<Diagnostic>> {
        if let Some(found) = self.named(expression) {
            return Ok(found);
        }
        if let Expr::Call {
            function,
            arguments,
            keywords,
            span,
        } = expression
            && self.names.member(function) == Some("POINTER")
        {
            let ([target], []) = (arguments.as_slice(), keywords.as_slice()) else {
                return Err(Box::new(refused(
                    *span,
                    "`POINTER` takes one `ctypes` type".to_owned(),
                    RuleId::FFI_03,
                    "name the type pointed to, as `ctypes.POINTER(ctypes.c_int)`",
                )));
            };
            let target = self.declared(target, true)?;
            if !target.is_item() {
                return Err(Box::new(pointer_of_pointer(*span)));
            }
            return Ok(CData::Pointer(Box::new(target)));
        }
        if arrays
            && let Expr::BinOp {
                left,
                operator: BinaryOperator::Mult,
                right,
                span,
            } = expression
        {
            return self.array(left, right, *span);
        }
        self.names
            .member(expression)
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

    /// Returns the class a name stands for: one an earlier assignment binds, or a structure
    /// of this module or of one it imports.
    fn named(&self, expression: &Expr) -> Option<CData> {
        let (module, name) = match expression {
            Expr::Name { id, .. } => {
                if let Some(alias) = self.aliases.get(id) {
                    return Some(alias.clone());
                }
                let (module, name) = self.names.program.get(id)?;
                (module, name.as_ref()?)
            }
            Expr::Attribute {
                value, attribute, ..
            } => {
                let Expr::Name { id, .. } = value.as_ref() else {
                    return None;
                };
                match self.names.program.get(id)? {
                    (module, None) => (module, &attribute.name),
                    (_, Some(_)) => return None,
                }
            }
            _ => return None,
        };
        self.structures
            .iter()
            .find(|each| each.module == *module && each.name == *name)
            .map(|each| CData::Structure(Arc::clone(each)))
    }

    /// Returns the array class `left * right`, whose length is an integer literal on either
    /// side.
    fn array(&self, left: &Expr, right: &Expr, span: Span) -> Result<CData, Box<Diagnostic>> {
        let literal = |expression: &Expr| match expression {
            Expr::Constant {
                value: Constant::Int(digits),
                ..
            } => digits.parse::<u32>().ok(),
            _ => None,
        };
        let (item, length) = match (literal(left), literal(right)) {
            (_, Some(length)) => (left, length),
            (Some(length), None) => (right, length),
            (None, None) => {
                return Err(Box::new(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        span,
                        "an array whose length is not an integer literal is planned for TyPy \
                         1.0 (FFI-10)",
                    )
                    .with_rule(RuleId::FFI_10),
                ));
            }
        };
        let item = self.declared(item, true)?;
        if !item.is_item() {
            return Err(Box::new(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    "an array of pointers or arrays is planned for TyPy 1.0 (FFI-10)",
                )
                .with_rule(RuleId::FFI_10),
            ));
        }
        Ok(CData::Array(Box::new(item), length))
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

/// The module being read: its name, which a structure's `repr` shows, and its package,
/// against which a relative import is read.
pub(crate) struct ReadModule<'module> {
    pub(crate) name: &'module str,
    pub(crate) package: Option<&'module ModuleName>,
}

/// Returns the declarations of a module, removes its assignments to `argtypes` and
/// `restype` from `body`, and adds its structures to `structures`, the program's, each class
/// statement becoming the binding of its name to the class.
pub(crate) fn read(
    module: &ReadModule<'_>,
    body: &mut Vec<Stmt>,
    structures: &mut Vec<Arc<StructureClass>>,
    diagnostics: &mut Diagnostics,
) -> ModuleDeclarations {
    let names = CtypesNames::of(body, module.package);
    if names.modules.is_empty() && names.members.is_empty() {
        return ModuleDeclarations::default();
    }
    let mut reader = Reader {
        module,
        names: &names,
        aliases: BTreeMap::new(),
        structures,
        declarations: ModuleDeclarations::default(),
        diagnostics,
    };
    for statement in std::mem::take(body) {
        if let Some(kept) = reader.statement(statement) {
            body.push(kept);
        }
    }
    reader.declarations
}

/// What reading a module keeps between its statements.
struct Reader<'read, 'module> {
    module: &'read ReadModule<'module>,
    names: &'read CtypesNames,
    /// The names bound to a type so far.
    aliases: BTreeMap<String, CData>,
    structures: &'read mut Vec<Arc<StructureClass>>,
    declarations: ModuleDeclarations,
    diagnostics: &'read mut Diagnostics,
}

impl Reader<'_, '_> {
    fn types(&self) -> Types<'_> {
        Types {
            names: self.names,
            aliases: &self.aliases,
            structures: self.structures,
        }
    }

    /// Reads one statement at module level, and returns what stays of it in the module.
    fn statement(&mut self, statement: Stmt) -> Option<Stmt> {
        match &statement {
            Stmt::ClassDef {
                name,
                bases,
                body,
                decorators,
                keyword_count,
                type_parameters,
                span,
            } if bases.len() == 1 && self.names.member(&bases[0]) == Some("Structure") => {
                let class = structure::ClassParts {
                    body,
                    decorated: !decorators.is_empty()
                        || *keyword_count > 0
                        || !type_parameters.is_empty(),
                    span: *span,
                };
                Some(self.structure(name, &class))
            }
            Stmt::ClassDef { bases, span, .. } if let Some(what) = self.planned_base(bases) => {
                self.diagnostics.push(structure::planned(*span, what));
                None
            }
            Stmt::Assign { targets, value, .. } => self
                .assignment(targets, value, statement.span())
                .then_some(statement),
            _ => Some(statement),
        }
    }

    /// Returns what a class statement with `bases` is, when it is a form of a `ctypes` class
    /// planned for 1.0: a `Union`, a structure with other bases, or a subclass of a structure.
    fn planned_base(&self, bases: &[Expr]) -> Option<&'static str> {
        let types = self.types();
        bases.iter().find_map(|base| match self.names.member(base) {
            Some("Union") => Some("a `ctypes.Union` subclass"),
            Some("Structure") => Some("a `Structure` subclass with other bases"),
            _ => matches!(types.named(base), Some(CData::Structure(_)))
                .then_some("a subclass of a `Structure` subclass"),
        })
    }

    /// Reads a structure class statement, and returns the binding of its name to the class.
    fn structure(
        &mut self,
        name: &typy_source::syntax::Identifier,
        class: &structure::ClassParts<'_>,
    ) -> Stmt {
        let span = &class.span;
        let id = u32::try_from(self.structures.len()).unwrap_or(u32::MAX);
        let types = Types {
            names: self.names,
            aliases: &self.aliases,
            structures: self.structures,
        };
        let fields = structure::fields(&types, class, self.diagnostics);
        let class = Arc::new(StructureClass::new(
            id,
            &name.name,
            self.module.name,
            fields,
        ));
        self.structures.push(Arc::clone(&class));
        self.aliases
            .insert(name.name.clone(), CData::Structure(class));
        Stmt::Assign {
            targets: vec![Expr::Name {
                id: name.name.clone(),
                span: name.span,
            }],
            value: Expr::Name {
                id: structure::hidden_name(id),
                span: *span,
            },
            span: *span,
        }
    }

    /// Reads an assignment at module level, and returns whether it stays in the module: a
    /// declaration of `argtypes` or `restype` leaves it.
    fn assignment(&mut self, targets: &[Expr], value: &Expr, span: Span) -> bool {
        // A name bound to a type stands for it in a later declaration; the assignment
        // stays, since the name is also a class the program calls.
        if let [Expr::Name { id, .. }] = targets
            && let Ok(ty) = self.types().declared(value, true)
        {
            self.aliases.insert(id.clone(), ty);
            return true;
        }
        if let [Expr::Name { id, .. }] = targets
            && let Expr::Call {
                function,
                arguments,
                keywords,
                span,
            } = value
            && self.names.loads_library(function)
        {
            library(
                id,
                arguments,
                keywords,
                *span,
                &mut self.declarations,
                self.diagnostics,
            );
            return true;
        }
        if let [target] = targets
            && let Some((variable, function, declares)) = declaration_target(target)
            && self
                .declarations
                .libraries
                .iter()
                .any(|library| library.variable == variable)
        {
            let types = Types {
                names: self.names,
                aliases: &self.aliases,
                structures: self.structures,
            };
            let declared = declaration(&types, value, declares, span, self.diagnostics);
            record(
                &mut self.declarations,
                (variable, function),
                declares,
                declared,
                span,
                self.diagnostics,
            );
            return false;
        }
        true
    }
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

/// Reads the value of an assignment to `argtypes` or `restype` with the types known before
/// it; an array is not one of them.
fn declaration(
    types: &Types<'_>,
    value: &Expr,
    declares: Declares,
    span: Span,
    diagnostics: &mut Diagnostics,
) -> Option<Declared> {
    let c_type = |expression: &Expr, diagnostics: &mut Diagnostics| match types
        .declared(expression, false)
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
