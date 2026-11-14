//! Calls of generic functions: the type arguments are solved from the arguments, checked
//! against each parameter's bound or constraints, and the call goes to the copy for them
//! (GEN-01, GEN-02, GEN-03, ADR-0023).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use super::generic_copy::Requested;
use crate::generic::{TypeParameter, infer};
use crate::typed::{FunctionId, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `name(arguments)` of the generic function `function`.
    pub(super) fn generic_call(
        &mut self,
        function: FunctionId,
        generic: &super::generic_copy::Generic,
        name: &str,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        if generic.module != self.program.module {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    format!(
                        "calling `{name}`, a generic function of another module, is planned for \
                         TyPy 0.5 (GEN-07)"
                    ),
                )
                .with_rule(RuleId::GEN_07),
                span,
            );
        }
        let solved = self.solve(function, &generic.parameters, arguments, keywords);
        let mut types = Vec::new();
        for (parameter, solution) in generic.parameters.iter().zip(solved) {
            match self.type_argument(parameter, solution, name, span) {
                Some(ty) => types.push(ty),
                None => return Self::invalid_call(span),
            }
        }
        // Inside a generic body a call may solve a parameter to another one; that body is
        // checked, never run, so its call goes to the generic itself.
        let target = if types.iter().any(|ty| ty.has_parameter() || ty.has_error()) {
            function
        } else {
            match self.program.request_copy(function, types.clone()) {
                Requested::Copy(copy) => copy,
                Requested::TooDeep => return self.copies_too_deep(name, span),
            }
        };
        let signature = self.program.instantiated(function, &types);
        match self.bind_to(&signature, 0, name, arguments, keywords, span) {
            Ok(bound) => Self::typed(
                TypedExprKind::Call {
                    function: target,
                    arguments: bound,
                    dispatch: false,
                },
                signature.returns,
                span,
            ),
            Err(error) => error,
        }
    }

    /// Solves the type parameters from the types of the arguments, each typed once without
    /// an expected type; what that first look reports is dropped, since the call is then
    /// checked against the solved signature.
    fn solve(
        &mut self,
        function: FunctionId,
        parameters: &[TypeParameter],
        arguments: &[Expr],
        keywords: &[Keyword],
    ) -> Vec<Option<Type>> {
        let signature = self.program.signatures[function.0 as usize].clone();
        let owner = parameters.first().map(|parameter| parameter.owner);
        let mut solved = vec![None; parameters.len()];
        let reported = self.diagnostics.len();
        let before = self.assigned.clone();
        let named = keywords.iter().filter_map(|keyword| {
            let name = keyword.name.as_ref()?;
            let position = signature
                .parameters
                .iter()
                .position(|parameter| parameter.name == name.name)?;
            Some((position, &keyword.value))
        });
        let given: Vec<(usize, &Expr)> = arguments.iter().enumerate().chain(named).collect();
        for (position, argument) in given {
            let (Some(parameter), Some(owner)) = (signature.parameters.get(position), owner) else {
                continue;
            };
            if parameter.ty.has_parameter() {
                let value = self.expression(argument);
                infer(&parameter.ty, &value.ty, owner, &mut solved);
            }
        }
        self.diagnostics.truncate(reported);
        self.assigned = before;
        solved
    }

    /// Returns the type argument of `parameter`: its solution, made one of its constraints,
    /// after checking its bound; `None` after reporting one that does not fit (GEN-02).
    fn type_argument(
        &mut self,
        parameter: &TypeParameter,
        solution: Option<Type>,
        name: &str,
        span: Span,
    ) -> Option<Type> {
        let Some(solution) = solution else {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "no argument of this call of `{name}` gives its type parameter `{}`",
                        parameter.name
                    ),
                )
                .with_rule(RuleId::GEN_01)
                .with_help("pass a value whose type gives it"),
            );
            return None;
        };
        if solution.has_parameter() || solution.has_error() {
            return Some(solution);
        }
        if !parameter.constraints.is_empty() {
            let constraint = parameter
                .constraints
                .iter()
                .find(|constraint| solution.is_assignable_to(constraint));
            if let Some(constraint) = constraint {
                return Some(constraint.clone());
            }
            let allowed: Vec<String> = parameter
                .constraints
                .iter()
                .map(ToString::to_string)
                .collect();
            self.unfit_argument(
                parameter,
                &solution,
                &format!("one of {}", allowed.join(", ")),
                name,
                span,
            );
            return None;
        }
        if let Some(bound) = &parameter.bound
            && !solution.is_assignable_to(bound)
        {
            self.unfit_argument(parameter, &solution, &bound.with_article(), name, span);
            return None;
        }
        Some(solution)
    }

    /// Reports a type argument outside its parameter's bound or constraints (GEN-02).
    fn unfit_argument(
        &mut self,
        parameter: &TypeParameter,
        solution: &Type,
        wanted: &str,
        name: &str,
        span: Span,
    ) {
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                span,
                format!(
                    "the type parameter `{}` of `{name}` takes {wanted}, and this call gives {}",
                    parameter.name,
                    solution.with_article()
                ),
            )
            .with_rule(RuleId::GEN_02),
        );
    }

    /// Reports a chain of copies past `MOST_NESTED_COPIES`, which would never end (ADR-0023).
    fn copies_too_deep(&mut self, name: &str, span: Span) -> TypedExpr {
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(428),
                span,
                format!(
                    "this call of `{name}` needs a copy of it inside {} others, each for a \
                     larger type, so making copies would never end",
                    super::generic_copy::MOST_NESTED_COPIES
                ),
            )
            .with_rule(RuleId::GEN_03)
            .with_help("a generic function cannot call itself with a larger type each time"),
            span,
        )
    }
}
