//! The functions a library exports: those `__all__` of its module lists, each with C types
//! for its parameters and result (FFI-05, ADR-0032, ADR-0044).

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span, TextOffset};
use typy_source::syntax::{Constant, Expr, Stmt};
use typy_source::{Module, Project};

use crate::typed::{FunctionId, TypedFunction, TypedProgram};
use crate::types::Type;

/// A function a library exports, under its own name as a C symbol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Export {
    /// The function's name, which is its C symbol.
    pub name: String,
    /// The function.
    pub function: FunctionId,
}

/// The C library functions the runtime calls, and `main`. A library that defined one of
/// them would call its own export in its place, so none is an export's name; a test of the
/// command line compares the list with what the runtime archive leaves undefined on each
/// host. On AArch64, Rust's `compiler_builtins` in the archive calls `getauxval` to learn
/// which atomic instructions the processor has; on macOS the runtime reads `errno` through
/// `__error`, an error's text from `strerror`, and the calling thread's stack from the
/// `pthread` functions (ADR-0057), and Rust's `core` there calls `bzero`, `memcmp` and
/// `sysctlbyname`.
pub const RUNTIME_C_FUNCTIONS: [&str; 64] = [
    "__error",
    "abort",
    "acos",
    "acosh",
    "asin",
    "asinh",
    "atan",
    "atan2",
    "atanh",
    "bcmp",
    "bzero",
    "cbrt",
    "ceil",
    "close",
    "cos",
    "cosh",
    "erf",
    "erfc",
    "exit",
    "exp",
    "exp2",
    "expm1",
    "floor",
    "fma",
    "fmod",
    "free",
    "frexp",
    "fstat",
    "ftruncate",
    "getauxval",
    "getrlimit",
    "isatty",
    "ldexp",
    "log",
    "log10",
    "log1p",
    "log2",
    "lseek",
    "main",
    "malloc",
    "memcmp",
    "memcpy",
    "memmove",
    "memset",
    "modf",
    "nextafter",
    "open",
    "pow",
    "pthread_get_stackaddr_np",
    "pthread_get_stacksize_np",
    "pthread_self",
    "read",
    "realloc",
    "round",
    "sin",
    "sinh",
    "sqrt",
    "strerror",
    "strlen",
    "sysctlbyname",
    "tan",
    "tanh",
    "trunc",
    "write",
];

/// The prefix of the runtime's symbols.
const RUNTIME_PREFIX: &str = "typy_";

pub(crate) fn refused(span: Span, message: String, help: &str) -> Box<Diagnostic> {
    Box::new(
        Diagnostic::new(DiagnosticCode::error(429), span, message)
            .with_rule(RuleId::FFI_05)
            .with_help(help.to_owned()),
    )
}

/// Returns the exports of the main module of `project`, a library, or a diagnostic for each
/// name of `__all__` that cannot be one.
pub(crate) fn exports(
    project: &Project,
    program: &TypedProgram,
) -> Result<Vec<Export>, Diagnostics> {
    let module = project.main_module();
    let mut diagnostics = Diagnostics::new();
    let names = match listed_names(module) {
        Ok(names) => names,
        Err(diagnostic) => return Err(Diagnostics::from(*diagnostic)),
    };
    let allocator: Vec<&str> = project
        .settings()
        .allocator
        .iter()
        .flat_map(|allocator| {
            [
                allocator.allocate.as_str(),
                allocator.reallocate.as_str(),
                allocator.free.as_str(),
            ]
        })
        .collect();
    let mut exports: Vec<Export> = Vec::new();
    for (name, span) in names {
        if exports.iter().any(|export| export.name == name) {
            diagnostics.push(*refused(
                span,
                format!("`__all__` lists `{name}` twice"),
                "list each export once",
            ));
            continue;
        }
        if let Err(diagnostic) = symbol_name(&name, span, &allocator) {
            diagnostics.push(*diagnostic);
            continue;
        }
        match exported_function(program, module.name.as_str(), &name, span) {
            Ok(function) => exports.push(Export { name, function }),
            Err(diagnostic) => diagnostics.push(*diagnostic),
        }
    }
    if diagnostics.has_errors() {
        return Err(diagnostics);
    }
    Ok(exports)
}

/// Returns the names `__all__` lists, each with the span of its string, from the one
/// statement at module level that binds it to a list or tuple of string literals.
fn listed_names(module: &Module) -> Result<Vec<(String, Span)>, Box<Diagnostic>> {
    let mut bindings = module_bindings(module, "__all__").into_iter();
    let Some((value, span)) = bindings.next() else {
        return Err(refused(
            Span::empty(module.file, TextOffset(0)),
            "a library's module lists the functions it exports in `__all__`, and this one \
             has none"
                .to_owned(),
            "add `__all__ = [\"name\"]` with the name of each function to export",
        ));
    };
    if let Some((_, again)) = bindings.next() {
        return Err(refused(
            again,
            "a library binds `__all__` once, and this binds it again".to_owned(),
            "list every export in the one assignment to `__all__`",
        ));
    }
    let Some(Expr::List { elements, .. } | Expr::Tuple { elements, .. }) = value else {
        return Err(refused(
            value.map_or(span, Expr::span),
            "a library's `__all__` is a list or tuple of string literals".to_owned(),
            "write the names as literals, such as `__all__ = [\"area\", \"scale\"]`",
        ));
    };
    elements
        .iter()
        .map(|element| match element {
            Expr::Constant {
                value: Constant::Str(name),
                span,
            } => Ok((name.clone(), *span)),
            other => Err(refused(
                other.span(),
                "a library's `__all__` is a list or tuple of string literals".to_owned(),
                "write the names as literals, such as `__all__ = [\"area\", \"scale\"]`",
            )),
        })
        .collect()
}

/// Returns each statement at module level that binds `name`: the value of an assignment,
/// none for an annotation without a value or an augmented assignment, and its span.
pub(crate) fn module_bindings<'module>(
    module: &'module Module,
    name: &str,
) -> Vec<(Option<&'module Expr>, Span)> {
    let names = |target: &Expr| matches!(target, Expr::Name { id, .. } if id == name);
    module
        .syntax
        .body
        .iter()
        .filter_map(|statement| match statement {
            Stmt::Assign {
                targets,
                value,
                span,
            } => targets.iter().any(names).then_some((Some(value), *span)),
            Stmt::AnnAssign {
                target,
                value,
                span,
                ..
            } if names(target) => Some((value.as_ref(), *span)),
            Stmt::AugAssign { target, span, .. } if names(target) => Some((None, *span)),
            _ => None,
        })
        .collect()
}

/// Checks that an export's name can be a C symbol of its own: ASCII, and none of the names
/// the runtime, TyPy or the project's allocator uses.
fn symbol_name(name: &str, span: Span, allocator: &[&str]) -> Result<(), Box<Diagnostic>> {
    if !name.is_ascii() {
        return Err(refused(
            span,
            format!("`{name}` is not an ASCII name, which a C symbol needs"),
            "export the function under an ASCII name",
        ));
    }
    // C keeps names that start with two underscores, or one and a capital letter, for its
    // implementation, where TyPy's entry and program functions are named too.
    let mut characters = name.chars();
    let reserved = match (characters.next(), characters.next()) {
        (Some('_'), Some(second)) => second == '_' || second.is_ascii_uppercase(),
        _ => false,
    };
    if reserved || name.starts_with(RUNTIME_PREFIX) {
        return Err(refused(
            span,
            format!(
                "`{name}` is a name C keeps for its implementation or TyPy keeps for its \
                 runtime"
            ),
            "export the function under a name that starts with a letter, and not with `typy_`",
        ));
    }
    if RUNTIME_C_FUNCTIONS.contains(&name) || allocator.contains(&name) {
        return Err(refused(
            span,
            format!(
                "`{name}` is a C function the library itself calls, which an export of that \
                 name would take the place of"
            ),
            "export the function under another name",
        ));
    }
    Ok(())
}

/// Returns the function the module defines under `name`, when its parameters and result
/// have C types.
fn exported_function(
    program: &TypedProgram,
    module: &str,
    name: &str,
    span: Span,
) -> Result<FunctionId, Box<Diagnostic>> {
    let Some(function) = program.functions.iter().find(|function| {
        function.module == module && function.qualified_name == name && function.name == name
    }) else {
        return Err(refused(
            span,
            format!("`__all__` lists `{name}`, which is not a function the module defines"),
            "list only functions defined with `def` at the module's top level",
        ));
    };
    if function.generic {
        return Err(refused(
            function.span,
            format!("`{name}` is generic, and an export has one C type for each parameter"),
            "export a function of `int`, `float` and `bool` that calls the generic one",
        ));
    }
    for parameter in &function.signature.parameters {
        if !has_c_type(&parameter.ty) {
            let what = format!("parameter `{}`", parameter.name);
            return Err(c_type_refused(function, &parameter.ty, &what));
        }
    }
    let returns = &function.signature.returns;
    if *returns != Type::NoneType && !has_c_type(returns) {
        return Err(c_type_refused(function, returns, "result"));
    }
    Ok(function.id)
}

fn c_type_refused(function: &TypedFunction, ty: &Type, what: &str) -> Box<Diagnostic> {
    refused(
        function.span,
        format!(
            "the {what} of `{}` is `{ty}`, which has no C type; an export takes and returns \
             `int`, `float` and `bool`, and may return `None`",
            function.name
        ),
        "convert other values inside the export, or keep the function out of `__all__`",
    )
}

/// Returns whether an export's parameter or result of type `ty` has a C type: `int`,
/// `float` and `bool` are `int64_t`, `double` and `bool` (ADR-0032).
const fn has_c_type(ty: &Type) -> bool {
    matches!(ty, Type::Int | Type::Float | Type::Bool)
}
