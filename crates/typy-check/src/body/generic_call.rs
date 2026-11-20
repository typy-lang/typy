//! Calls of generic functions: the type arguments are solved from the arguments, checked
//! against each parameter's bound or constraints, and the call goes to the copy for them
//! (GEN-01, GEN-02, GEN-03, ADR-0023).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use super::binding::{Parameters, Slot};
use super::generic_copy::Requested;
use crate::generic::{TypeParameter, infer};
use crate::typed::{ClassId, FunctionId, FunctionSignature, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `name(arguments)` of the generic function `function`.
    pub(super) fn generic_call(
        &mut self,
        function: FunctionId,
        name: &str,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let (target, signature) =
            match self.call_target(function, 0, name, arguments, keywords, span) {
                Ok(found) => found,
                Err(error) => return error,
            };
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

    /// Returns the function a call of `function` runs, with its signature: for a generic
    /// function, the copy for the type arguments that the arguments after the first `skip`
    /// give; `function` itself for any other.
    pub(super) fn call_target(
        &mut self,
        function: FunctionId,
        skip: usize,
        name: &str,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Result<(FunctionId, FunctionSignature), TypedExpr> {
        self.call_target_with(function, (skip, &[]), name, (arguments, keywords), span)
    }

    /// Returns the function a call of `function` runs, as `call_target` does, with the first
    /// type arguments `known`: those of the copy of a generic class whose method it is, for a
    /// method with type parameters of its own (GEN-09).
    pub(super) fn call_target_with(
        &mut self,
        function: FunctionId,
        (skip, known): (usize, &[Type]),
        name: &str,
        (arguments, keywords): (&[Expr], &[Keyword]),
        span: Span,
    ) -> Result<(FunctionId, FunctionSignature), TypedExpr> {
        if self.program.generic(function).is_none() {
            return Ok((
                function,
                self.program.signatures[function.0 as usize].clone(),
            ));
        }
        let given = (arguments, keywords);
        let types = self.solved_arguments(function, (skip, known), name, given, span)?;
        // Inside a generic body a call may solve a parameter to another one; that body is
        // checked, never run, so its call goes to the generic itself.
        let target = if types.iter().any(|ty| ty.has_parameter() || ty.has_error()) {
            function
        } else {
            let class = self.make_owner_copy(function, &types);
            match self.program.request_copy(function, types.clone(), class) {
                Requested::Copy(copy) => copy,
                Requested::TooDeep => return Err(self.copies_too_deep(name, span)),
            }
        };
        let signature = self.program.instantiated(function, &types);
        self.program.make_class_copies();
        Ok((target, signature))
    }

    /// Makes the copy of the generic class whose type parameters `function` is generic over,
    /// first among its own, if it is a method of one, and returns it, so the copy of the
    /// method belongs to it (ADR-0028).
    fn make_owner_copy(&mut self, function: FunctionId, types: &[Type]) -> Option<ClassId> {
        let parameters = self
            .program
            .generic(function)
            .map(|generic| generic.parameters.clone())?;
        let (owner, count) = self
            .program
            .class_copies
            .borrow()
            .roots
            .iter()
            .find(|(_, generic)| parameters.starts_with(&generic.parameters))
            .map(|(class, generic)| (*class, generic.parameters.len()))?;
        let copy = self
            .program
            .class_copies
            .borrow_mut()
            .applied(owner, types[..count].to_vec());
        self.program.make_class_copies();
        copy.class_id()
    }

    /// Returns the type arguments of the generic function `function`: the first `known`, then
    /// those the arguments after the first `skip` give, each checked against its bound or
    /// constraints.
    pub(super) fn solved_arguments(
        &mut self,
        function: FunctionId,
        (skip, known): (usize, &[Type]),
        name: &str,
        (arguments, keywords): (&[Expr], &[Keyword]),
        span: Span,
    ) -> Result<Vec<Type>, TypedExpr> {
        let parameters = self
            .program
            .generic(function)
            .map(|generic| generic.parameters.clone())
            .unwrap_or_default();
        let solved = self.solve(function, skip, &parameters, arguments, keywords);
        let mut types: Vec<Type> = known.to_vec();
        for (parameter, solution) in parameters.iter().zip(solved).skip(known.len()) {
            match self.type_argument(parameter, solution, name, span) {
                Some(ty) => types.push(ty),
                None => return Err(Self::invalid_call(span)),
            }
        }
        Ok(types)
    }

    /// Solves the type parameters from the types of the arguments, each typed once without
    /// an expected type; what that first look reports is dropped, since the call is then
    /// checked against the solved signature.
    fn solve(
        &mut self,
        function: FunctionId,
        skip: usize,
        parameters: &[TypeParameter],
        arguments: &[Expr],
        keywords: &[Keyword],
    ) -> Vec<Option<Type>> {
        let signature = self.program.signatures[function.0 as usize].clone();
        let mut pairs: Vec<(Type, Type)> = Vec::new();
        let reported = self.diagnostics.len();
        let before = self.assigned.clone();
        // Each argument is paired with the type it must have: its parameter's, or the type of
        // each item of `*args` or `**kwargs`; a value `*` or `**` unpacks, with each type it
        // may have to fit (EXP-17, ADR-0073).
        let shape = Parameters::of(&signature.parameters[skip.min(signature.parameters.len())..]);
        let mut position = 0;
        let mut tail = false;
        for argument in arguments {
            let (value, unpacked) = match argument {
                Expr::Starred { value, .. } => (value.as_ref(), true),
                other => (other, false),
            };
            let reached: Vec<Type> = if tail || unpacked {
                shape.reached_from(position).into_iter().cloned().collect()
            } else {
                shape
                    .of_position(position)
                    .and_then(Slot::expected)
                    .into_iter()
                    .cloned()
                    .collect()
            };
            if reached.iter().all(|expected| !expected.has_parameter()) {
                position += 1;
                continue;
            }
            let ty = Self::version_tuple(self.expression(value)).ty;
            match (&ty, unpacked) {
                (Type::Tuple(items), true) if !tail => {
                    for item in items {
                        let expected = shape.of_position(position).and_then(Slot::expected);
                        pairs.extend(expected.map(|expected| (expected.clone(), item.clone())));
                        position += 1;
                    }
                    continue;
                }
                (_, true) => {
                    tail = true;
                    if let Some(item) = super::unpacked_arguments::iterated_type(&ty) {
                        pairs.extend(reached.into_iter().map(|expected| (expected, item.clone())));
                    }
                }
                (_, false) => {
                    pairs.extend(reached.into_iter().map(|expected| (expected, ty.clone())));
                }
            }
            position += 1;
        }
        for keyword in keywords {
            let Some(name) = &keyword.name else {
                let ty = self.expression(&keyword.value).ty;
                if let Type::Dict(_, item) = &ty {
                    pairs.extend(
                        shape
                            .reached_by_name()
                            .into_iter()
                            .map(|expected| (expected.clone(), item.as_ref().clone())),
                    );
                }
                continue;
            };
            let Some(expected) = shape.of_keyword(&name.name).and_then(Slot::expected) else {
                continue;
            };
            if expected.has_parameter() {
                let value = self.expression(&keyword.value);
                pairs.push((expected.clone(), value.ty));
            }
        }
        self.diagnostics.truncate(reported);
        self.assigned = before;
        solve_each(parameters, &pairs)
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
            // A parameter of a generic class may also be given with the class.
            let of_class = self
                .program
                .class_copies
                .borrow()
                .roots
                .values()
                .any(|generic| generic.parameters.contains(parameter));
            let help = match name.split_once('.') {
                Some((class, method)) if of_class => {
                    format!("give the class its type arguments, as in `{class}[int].{method}(...)`")
                }
                _ => "pass a value whose type gives it".to_owned(),
            };
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
                .with_help(help),
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
        // A protocol is satisfied by any class with its members (GEN-04).
        if let Some(protocol) = parameter
            .bound
            .as_ref()
            .and_then(|bound| self.bound_protocol(bound))
        {
            if let Some(reason) = self.unsatisfied(&solution, protocol) {
                let protocol = self.program.classes[protocol.0 as usize].name.clone();
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        span,
                        format!(
                            "this call of `{name}` gives {}, which does not satisfy the protocol \
                             `{protocol}`: {reason}",
                            solution.with_article()
                        ),
                    )
                    .with_rule(RuleId::GEN_04),
                );
                return None;
            }
            return Some(solution);
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

    /// Returns the copy of the generic class `class` a call makes, its type arguments solved
    /// from the arguments of `__init__` as a generic function's are; `class` itself for any
    /// other class (GEN-01, ADR-0028).
    pub(super) fn class_to_call(
        &mut self,
        class: crate::typed::ClassId,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Result<crate::typed::ClassId, TypedExpr> {
        let Some(generic) = self
            .program
            .class_copies
            .borrow()
            .roots
            .get(&class)
            .cloned()
        else {
            return Ok(class);
        };
        let name = generic.instance.name.clone();
        let solved = match self.program.method(class, "__init__") {
            Some(init) => self.solve(init, 1, &generic.parameters, arguments, keywords),
            None => vec![None; generic.parameters.len()],
        };
        let mut types = Vec::new();
        for (parameter, solution) in generic.parameters.iter().zip(solved) {
            match self.type_argument(parameter, solution, &name, span) {
                Some(ty) => types.push(ty),
                None => return Err(Self::invalid_call(span)),
            }
        }
        let instance = self.program.class_copies.borrow_mut().applied(class, types);
        self.program.make_class_copies();
        Ok(instance.class_id().unwrap_or(class))
    }

    /// Checks `Class[arguments](...)`: a call of the copy of a generic class for the type
    /// arguments it is given (GEN-01, ADR-0028).
    pub(super) fn applied_class_call(
        &mut self,
        function: &Expr,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        if let Type::Instance(class) = self.annotation(function) {
            return self.new_instance(class.id, arguments, keywords, span);
        }
        for argument in arguments {
            self.expression(argument);
        }
        Self::invalid_call(span)
    }
}

/// Solves `parameters` from each pair of a parameter's type and its argument's: the parameters
/// of each definition apart, as a method of a generic class with type parameters of its own has
/// two (GEN-09).
fn solve_each(parameters: &[TypeParameter], pairs: &[(Type, Type)]) -> Vec<Option<Type>> {
    let mut solved = vec![None; parameters.len()];
    let mut owners: Vec<Span> = Vec::new();
    for parameter in parameters {
        if !owners.contains(&parameter.owner) {
            owners.push(parameter.owner);
        }
    }
    for owner in owners {
        let count = parameters
            .iter()
            .filter(|parameter| parameter.owner == owner)
            .count();
        let mut found = vec![None; count];
        for (parameter, argument) in pairs {
            infer(parameter, argument, owner, &mut found);
        }
        for (position, parameter) in parameters.iter().enumerate() {
            if parameter.owner == owner {
                solved[position] = found.get_mut(parameter.index).and_then(Option::take);
            }
        }
    }
    solved
}
