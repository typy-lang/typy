//! Visits every expression of a statement, outside nested definitions.

use typy_source::syntax::{Comprehension, Expr, Stmt};

pub(super) fn visit_statement(statement: &Stmt, visit: &mut dyn FnMut(&Expr)) {
    match statement {
        Stmt::Return {
            value: Some(value), ..
        }
        | Stmt::Expr { value, .. }
        | Stmt::AnnAssign {
            value: Some(value), ..
        } => visit_expression(value, visit),
        Stmt::Assign { targets, value, .. } => {
            for target in targets {
                visit_expression(target, visit);
            }
            visit_expression(value, visit);
        }
        Stmt::AugAssign { target, value, .. } => {
            visit_expression(target, visit);
            visit_expression(value, visit);
        }
        Stmt::For {
            iter, body, orelse, ..
        } => {
            visit_expression(iter, visit);
            body.iter()
                .chain(orelse)
                .for_each(|inner| visit_statement(inner, visit));
        }
        Stmt::While {
            test, body, orelse, ..
        }
        | Stmt::If {
            test, body, orelse, ..
        } => {
            visit_expression(test, visit);
            body.iter()
                .chain(orelse)
                .for_each(|inner| visit_statement(inner, visit));
        }
        _ => {}
    }
}

pub(super) fn visit_expression(expression: &Expr, visit: &mut dyn FnMut(&Expr)) {
    visit(expression);
    match expression {
        Expr::BoolOp { values, .. }
        | Expr::Tuple {
            elements: values, ..
        }
        | Expr::List {
            elements: values, ..
        }
        | Expr::Set {
            elements: values, ..
        } => values
            .iter()
            .for_each(|value| visit_expression(value, visit)),
        Expr::ListComp {
            element,
            generators,
            ..
        }
        | Expr::SetComp {
            element,
            generators,
            ..
        } => {
            visit_expression(element, visit);
            visit_generators(generators, visit);
        }
        Expr::DictComp {
            key,
            value,
            generators,
            ..
        } => {
            visit_expression(key, visit);
            visit_expression(value, visit);
            visit_generators(generators, visit);
        }
        Expr::Dict { keys, values, .. } => {
            keys.iter()
                .flatten()
                .chain(values)
                .for_each(|value| visit_expression(value, visit));
        }
        Expr::BinOp { left, right, .. } => {
            visit_expression(left, visit);
            visit_expression(right, visit);
        }
        Expr::UnaryOp { operand, .. } | Expr::Attribute { value: operand, .. } => {
            visit_expression(operand, visit);
        }
        Expr::IfExp {
            test, body, orelse, ..
        } => {
            visit_expression(test, visit);
            visit_expression(body, visit);
            visit_expression(orelse, visit);
        }
        Expr::Compare {
            left, comparators, ..
        } => {
            visit_expression(left, visit);
            for comparator in comparators {
                visit_expression(comparator, visit);
            }
        }
        Expr::Call {
            function,
            arguments,
            keywords,
            ..
        } => {
            visit_expression(function, visit);
            for argument in arguments {
                visit_expression(argument, visit);
            }
            for keyword in keywords {
                visit_expression(&keyword.value, visit);
            }
        }
        Expr::Subscript { value, index, .. } => {
            visit_expression(value, visit);
            visit_expression(index, visit);
        }
        Expr::Slice {
            lower, upper, step, ..
        } => {
            for bound in [lower, upper, step].into_iter().flatten() {
                visit_expression(bound, visit);
            }
        }
        Expr::Constant { .. } | Expr::Name { .. } | Expr::Quoted { .. } | Expr::Other { .. } => {}
    }
}

fn visit_generators(generators: &[Comprehension], visit: &mut dyn FnMut(&Expr)) {
    for generator in generators {
        visit_expression(&generator.target, visit);
        visit_expression(&generator.iter, visit);
        for condition in &generator.ifs {
            visit_expression(condition, visit);
        }
    }
}
