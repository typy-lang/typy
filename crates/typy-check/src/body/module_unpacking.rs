//! Calls of a function of a built-in module with `*` of a value only the run knows (EXP-22,
//! ADR-0083). A function of `math` or `sys` is checked for each number of arguments it takes,
//! as a built-in function is, with CPython 3.15's messages for any other number. `math.gcd`,
//! `math.lcm` and `math.hypot` take any number, so the runtime folds the list `*` gives. A
//! function of `ctypes` lays out C data from the arguments the program writes, so `*` there is
//! refused.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Comprehension, Expr, Keyword};

use super::BodyChecker;
use super::arity_table::{function_arity, irregular};
use super::builtin_module::ModuleFunction;
use super::builtin_unpacking::Arity;
use crate::math::{MathFunction, math_function_name};
use crate::typed::{TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `callee(arguments, keywords)`, a call of the function `function` of a built-in
    /// module with `*` of a value only the run knows; `None` for a call with `**`.
    pub(super) fn module_unpacked(
        &mut self,
        (callee, function): (&Expr, ModuleFunction),
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        if keywords.iter().any(|keyword| keyword.name.is_none()) {
            return None;
        }
        let (module, name) = match function {
            ModuleFunction::Math(math) => ("math", math_function_name(math)),
            ModuleFunction::Exit => ("sys", "exit"),
            ModuleFunction::GetRecursionLimit => ("sys", "getrecursionlimit"),
            ModuleFunction::SetRecursionLimit => ("sys", "setrecursionlimit"),
            _ => return Some(self.ctypes_unpacked(span)),
        };
        let (_, _, minimum, maximum, few, many) = function_arity(module, name)?;
        let shown = format!("{module}.{name}");
        let (ModuleFunction::Math(math), None) = (function, maximum) else {
            let arity = Arity::new(minimum, maximum.unwrap_or(minimum), few, many)
                .worded(irregular(module, name));
            return Some(self.arity_arms(
                (callee, &shown),
                &arity,
                Vec::new(),
                (arguments, keywords),
                span,
            ));
        };
        Some(self.math_of_list((math, &shown), arguments, keywords, span))
    }

    /// Checks `math.gcd(*values)`, `math.lcm(*values)` or `math.hypot(*values)`: every value,
    /// of one type, goes into a list the runtime folds, of `int`s for the first two and of
    /// `float`s for `hypot`, to which an `int` is converted.
    fn math_of_list(
        &mut self,
        (function, shown): (MathFunction, &str),
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        // The call takes what it unpacks, so its own errors stand (ADR-0073).
        self.unpacking_calls.push(span);
        let what = format!("the arguments of `{shown}` called with `*`");
        let (list, item) = self.one_type_values(arguments, &what, span);
        if let Some(keyword) = keywords.first() {
            let message = format!("`{shown}` takes no keyword arguments");
            return self.error(
                Diagnostic::new(DiagnosticCode::error(404), keyword.span, message)
                    .with_rule(RuleId::EXP_22),
                span,
            );
        }
        let wanted = if function == MathFunction::Hypot {
            Type::Float
        } else {
            Type::Int
        };
        let values = match &item {
            ty if *ty == wanted || ty.has_error() => list,
            Type::Int if wanted == Type::Float => self.floats_of(list, span),
            other => {
                let message = format!(
                    "`{shown}` takes values of type {wanted}, and `*` gives values of type {other}"
                );
                return self.error(
                    Diagnostic::new(DiagnosticCode::error(404), list.span, message)
                        .with_rule(RuleId::EXP_22),
                    span,
                );
            }
        };
        Self::typed(
            TypedExprKind::MathOfList {
                function,
                values: Box::new(values),
            },
            wanted,
            span,
        )
    }

    /// Returns `[float(each) for each in list]` of the list of `int`s `list`, which is
    /// evaluated first.
    fn floats_of(&mut self, list: TypedExpr, span: Span) -> TypedExpr {
        let ints = self.call_variable("unpacked", list.ty.clone(), span);
        let store = Self::stored(ints, list);
        // The comprehension binds its own variable, named as no program can name one.
        let each = Expr::Name {
            id: format!("<each {}>", self.local_order.len()),
            span,
        };
        let floats = Expr::ListComp {
            element: Box::new(Expr::Call {
                function: Box::new(Expr::Name {
                    id: "float".to_owned(),
                    span,
                }),
                arguments: vec![each.clone()],
                keywords: Vec::new(),
                span,
            }),
            generators: vec![Comprehension {
                target: each,
                iter: self.hidden_name(ints, span),
                ifs: Vec::new(),
                is_async: false,
                span,
            }],
            span,
        };
        let floats = self.expression(&floats);
        Self::in_order(vec![store], floats)
    }

    /// Returns E0202 for `*` of a value only the run knows in a call of a function of
    /// `ctypes`.
    fn ctypes_unpacked(&mut self, span: Span) -> TypedExpr {
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(202),
                span,
                "a function of `ctypes` lays out C data from the arguments the program writes, so \
                 `*` of a value only the run knows cannot give them",
            )
            .with_rule(RuleId::EXP_22)
            .with_help("pass the arguments one by one"),
            span,
        )
    }
}
