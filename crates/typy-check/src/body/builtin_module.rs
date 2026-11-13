//! Looks up the names of the built-in modules, `math` and `sys` (ADR-0018), for imports,
//! attributes and calls: a name newer than the program's Python floor is not there, and a
//! name TyPy plans or refuses says so.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::Alias;

use super::Program;
use super::import::Imported;
use crate::math::{MathFunction, MathName, math_name};
use crate::sys::{SysName, SysValue, sys_name};
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

/// The standard library modules TyPy provides as built-in modules.
const BUILTIN_MODULES: [&str; 2] = ["math", "sys"];

/// Returns whether `module` is a built-in module of TyPy.
pub(crate) fn is_builtin_module(module: &str) -> bool {
    BUILTIN_MODULES.contains(&module)
}

/// A function of a built-in module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModuleFunction {
    /// A function of `math`.
    Math(MathFunction),
    /// `sys.exit`.
    Exit,
}

/// A value of a built-in module.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BuiltinValue {
    /// A constant of `math`.
    Float(f64),
    /// A value of `sys`.
    Sys(SysValue),
}

impl BuiltinValue {
    /// Returns the value as an expression at `span`.
    pub(crate) fn expression(self, span: Span) -> TypedExpr {
        let (kind, ty) = match self {
            Self::Float(value) => (
                TypedExprKind::Constant(TypedConstant::Float(value)),
                Type::Float,
            ),
            Self::Sys(value) => (
                TypedExprKind::SysValue(value),
                match value {
                    SysValue::Argv => Type::List(Box::new(Type::Str)),
                    SysValue::Stdout | SysValue::Stderr => Type::TextFile,
                },
            ),
        };
        TypedExpr { kind, ty, span }
    }
}

/// What a name of a built-in module is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BuiltinName {
    Value(BuiltinValue),
    Function(ModuleFunction),
}

fn error(code: u16, span: Span, message: String, rule: RuleId) -> Box<Diagnostic> {
    Box::new(Diagnostic::new(DiagnosticCode::error(code), span, message).with_rule(rule))
}

impl Program {
    /// Returns what `name` is in the built-in module `module`, or why the program cannot use
    /// it.
    pub(crate) fn builtin_lookup(
        &self,
        module: &str,
        name: &str,
        span: Span,
    ) -> Result<BuiltinName, Box<Diagnostic>> {
        if module == "sys" {
            return Self::sys_lookup(name, span);
        }
        match math_name(name) {
            None => Err(error(
                425,
                span,
                format!("module `math` has no name `{name}`"),
                RuleId::LIB_05,
            )),
            Some(MathName::Planned) => Err(error(
                201,
                span,
                format!("`math.{name}` is planned for TyPy 0.4 (LIB-12)"),
                RuleId::LIB_12,
            )),
            Some(MathName::Function(function)) if function.since() > self.floor => Err(Box::new(
                Diagnostic::new(
                    DiagnosticCode::error(425),
                    span,
                    format!(
                        "module `math` has no name `{name}` before Python 3.{}",
                        function.since()
                    ),
                )
                .with_rule(RuleId::LIB_05)
                .with_help(format!(
                    "the program's Python floor is 3.{}; set `python-floor = \"3.{}\"` in \
                         typy.toml to use it",
                    self.floor,
                    function.since()
                )),
            )),
            Some(MathName::Function(function)) => {
                Ok(BuiltinName::Function(ModuleFunction::Math(function)))
            }
            Some(MathName::Constant(value)) => Ok(BuiltinName::Value(BuiltinValue::Float(value))),
        }
    }

    fn sys_lookup(name: &str, span: Span) -> Result<BuiltinName, Box<Diagnostic>> {
        match sys_name(name) {
            Some(SysName::Value(value)) => Ok(BuiltinName::Value(BuiltinValue::Sys(value))),
            Some(SysName::Exit) => Ok(BuiltinName::Function(ModuleFunction::Exit)),
            Some(SysName::LazySwitch) => Err(Box::new(
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    span,
                    format!(
                        "`sys.{name}` changes how imports behave from outside the module that \
                         imports, which TyPy does not compile (MOD-09)"
                    ),
                )
                .with_rule(RuleId::MOD_09)
                .with_help("make an import lazy in its own module, with `lazy import` or `__lazy_modules__`"),
            )),
            Some(SysName::Planned) => Err(error(
                201,
                span,
                format!("`sys.{name}` is planned for TyPy 0.4 (LIB-13)"),
                RuleId::LIB_13,
            )),
            Some(SysName::Unprovided) => Err(error(
                202,
                span,
                format!("TyPy does not provide `sys.{name}`"),
                RuleId::LIB_04,
            )),
            None => Err(error(
                425,
                span,
                format!("module `sys` has no name `{name}`"),
                RuleId::LIB_04,
            )),
        }
    }

    /// Returns what `from module import name` binds for a built-in module.
    pub(crate) fn builtin_from(
        &self,
        module: &str,
        alias: &Alias,
    ) -> Result<Imported, Box<Diagnostic>> {
        Ok(
            match self.builtin_lookup(module, &alias.name.name, alias.name.span)? {
                BuiltinName::Value(value) => Imported::ModuleValue(value),
                BuiltinName::Function(function) => Imported::ModuleFunction(function),
            },
        )
    }
}
