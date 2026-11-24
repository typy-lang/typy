//! Checks generator expressions (EXP-15): `(element for target in source if condition ...)`
//! is a nested generator function of `.0`, the iterator of its first iterable, which it is
//! called with where it runs, as CPython does. Its body is a `for` over `.0`, then a `for`
//! over each later iterable and an `if` for each condition, around `yield element`, so the
//! later iterables and the conditions run as the generator runs.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Comprehension, Expr, ParameterKind, Stmt};

use super::BodyChecker;
use super::closure_scope::{FIRST_ITERATOR, GENERATOR_EXPRESSION};
use crate::check::Definition;
use crate::typed::{
    FunctionId, FunctionSignature, SignatureParameter, TypedExpr, TypedExprKind, TypedStmt,
};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the generator the generator expression makes, of type `Generator[Y, None,
    /// None]`: `Y` is the item type `expected` gives, an iterable's or an iterator's, or else
    /// the element's type.
    pub(super) fn generator_expression(
        &mut self,
        element: &Expr,
        generators: &[Comprehension],
        expected: Option<&Type>,
        span: Span,
    ) -> TypedExpr {
        let Some(first) = generators.first() else {
            unreachable!("a generator expression has a `for` clause")
        };
        if let Some(refused) = self.refused_generator_expression(element, generators) {
            return refused;
        }
        // Its body is checked where it stands, as a `lambda`'s is (TYP-12).
        if let Some(refused) = self.refused_capture(span, "generator expression") {
            return refused;
        }
        // The first iterable runs where the expression runs (EXP-15).
        let source = self.iterator_of(&first.iter, first.iter.span());
        if source.ty.has_error() {
            return Self::invalid_call(span);
        }
        let function = self.program.next_function();
        let yields = expected.and_then(expected_item).unwrap_or(Type::Error);
        let signature = FunctionSignature {
            parameters: vec![SignatureParameter {
                name: FIRST_ITERATOR.to_owned(),
                ty: source.ty.clone(),
                default: None,
                default_call: None,
                kind: ParameterKind::PositionalOnly,
            }],
            returns: generator_of(yields.clone()),
        };
        self.program.signatures.push(signature.clone());
        self.program.functions.push(None);
        let mut names = self.captured_names(span);
        names.retain(|name| {
            self.locals
                .get(name)
                .is_none_or(|symbol| !self.is_module_variable(*symbol))
        });
        let (closure, captures) = self.capture_values(GENERATOR_EXPRESSION, function, &names, span);
        let qualified_name = if self.qualified_name.is_empty() {
            GENERATOR_EXPRESSION.to_owned()
        } else {
            format!("{}.<locals>.{GENERATOR_EXPRESSION}", self.qualified_name)
        };
        let statements = [generator_body(element, generators)];
        let definition = Definition {
            name: GENERATOR_EXPRESSION.to_owned(),
            qualified_name,
            class: None,
            takes_instance: false,
            copy_of: None,
            home: None,
            signature,
            type_parameters: Vec::new(),
            parameters: vec![(FIRST_ITERATOR.to_owned(), first.iter.span())],
            body: &statements,
            span,
        };
        self.program.closures.insert(function, closure.variables());
        Self::check_body_of(
            self.program,
            function,
            &definition,
            Some(closure),
            self.diagnostics,
        );
        let yields = if yields == Type::Error {
            self.settle_yields(function)
        } else {
            yields
        };
        let ty = generator_of(yields);
        let callable = Type::Callable {
            parameters: vec![source.ty.clone()],
            returns: Box::new(ty.clone()),
        };
        let made = Self::typed(TypedExprKind::Lambda { function, captures }, callable, span);
        Self::typed(
            TypedExprKind::CallValue {
                callee: Box::new(made),
                arguments: vec![source],
            },
            ty,
            span,
        )
    }

    /// Returns E0201 for a generator expression with an assignment expression, whose variable
    /// belongs to the function around it.
    fn refused_generator_expression(
        &mut self,
        element: &Expr,
        generators: &[Comprehension],
    ) -> Option<TypedExpr> {
        let mut named = None;
        let mut visit = |expression: &Expr| {
            if let Expr::Named { span, .. } = expression {
                named.get_or_insert(*span);
            }
        };
        crate::check::visit_expression(element, &mut visit);
        for (position, generator) in generators.iter().enumerate() {
            if position > 0 {
                crate::check::visit_expression(&generator.iter, &mut visit);
            }
            for condition in &generator.ifs {
                crate::check::visit_expression(condition, &mut visit);
            }
        }
        let at = named?;
        let diagnostic = Diagnostic::new(
            DiagnosticCode::error(201),
            at,
            "an assignment expression inside a generator expression is planned for TyPy 1.0 \
             (EXP-18)",
        )
        .with_rule(RuleId::EXP_18)
        .with_help("assign the variable in a statement before the generator expression");
        Some(self.error(diagnostic, at))
    }

    /// Returns the type of the element the generator expression `function` yields, which no
    /// context gave, and gives its signature and its generator types that item type.
    fn settle_yields(&mut self, function: FunctionId) -> Type {
        let index = function.0 as usize;
        let yields = self.program.functions[index]
            .as_ref()
            .and_then(|typed| yielded_type(&typed.body))
            .unwrap_or(Type::Error);
        self.program.signatures[index].returns = generator_of(yields.clone());
        if let Some(typed) = &mut self.program.functions[index] {
            typed.signature.returns = generator_of(yields.clone());
            if let Some(types) = &mut typed.generator {
                types.yields = yields.clone();
            }
        }
        yields
    }
}

/// Returns the item type an iterable, an iterator or a generator the context expects gives.
fn expected_item(expected: &Type) -> Option<Type> {
    match expected {
        Type::Iterable(item) | Type::Iterator(item) => Some(item.as_ref().clone()),
        Type::Generator { yields, .. } => Some(yields.as_ref().clone()),
        _ => None,
    }
}

/// Returns `Generator[yields, None, None]`.
fn generator_of(yields: Type) -> Type {
    Type::Generator {
        yields: Box::new(yields),
        sends: Box::new(Type::NoneType),
        returns: Box::new(Type::NoneType),
    }
}

/// Returns the body of a generator expression's function: a `for` over `.0` for the first
/// clause, a `for` over its iterable for each later one, and an `if` for each condition,
/// around `yield element`.
fn generator_body(element: &Expr, generators: &[Comprehension]) -> Stmt {
    let span = element.span();
    let mut inner = Stmt::Expr {
        value: Expr::Yield {
            value: Some(Box::new(element.clone())),
            span,
        },
        span,
    };
    for (position, generator) in generators.iter().enumerate().rev() {
        for condition in generator.ifs.iter().rev() {
            inner = Stmt::If {
                test: condition.clone(),
                body: vec![inner],
                orelse: Vec::new(),
                span: condition.span(),
            };
        }
        let iter = if position == 0 {
            Expr::Name {
                id: FIRST_ITERATOR.to_owned(),
                span: generator.iter.span(),
            }
        } else {
            generator.iter.clone()
        };
        inner = Stmt::For {
            target: generator.target.clone(),
            iter,
            body: vec![inner],
            orelse: Vec::new(),
            span: generator.span,
        };
    }
    inner
}

/// Returns the type of what the first `yield` among `body` gives.
fn yielded_type(body: &[TypedStmt]) -> Option<Type> {
    body.iter().find_map(|statement| match statement {
        TypedStmt::Yield { value, .. } => Some(value.ty.clone()),
        TypedStmt::ForEach { body, .. }
        | TypedStmt::ForRange { body, .. }
        | TypedStmt::If { body, .. } => yielded_type(body),
        _ => None,
    })
}
