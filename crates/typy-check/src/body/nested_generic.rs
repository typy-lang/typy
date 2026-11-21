//! Nested functions with type parameters of their own (FUN-18, ADR-0081). Each run of the
//! `def` makes one function object, which holds the captures every copy reads by the same
//! layout. The body is checked with the type parameters opaque, and once for each constraint,
//! as a generic function of a module is; then once for each set of types its calls give, each
//! a copy that a call by name runs with that object.

use typy_diagnostics::Diagnostics;

use super::generic_copy::GenericCopy;
use super::nested_function::Pending;
use super::{BodyChecker, Program};
use crate::check::Definition;
use crate::typed::FunctionId;
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks each nested function of `generic` with its type parameters opaque, then each
    /// copy the program asks for of any of them, until none is left, since a copy may ask for
    /// another.
    pub(super) fn check_nested_generics(
        program: &mut Program,
        generic: &[Pending],
        diagnostics: &mut Diagnostics,
    ) {
        for nested in generic {
            program
                .closures
                .insert(nested.function, nested.closure.variables());
            // Its body is checked, never run: no code is made for it or what it nests.
            let outer = program.generic_pass.replace(nested.function);
            for arguments in crate::check::combinations(&nested.definition.type_parameters) {
                Self::check_nested_body(
                    program,
                    nested,
                    (nested.function, &arguments),
                    diagnostics,
                );
            }
            program.generic_pass = outer;
        }
        while let Some((copy, nested)) = next_copy(program, generic) {
            program
                .closures
                .insert(copy.copy, nested.closure.variables());
            let depth = std::mem::replace(&mut program.generics.depth, copy.depth);
            Self::check_nested_body(program, nested, (copy.copy, &copy.arguments), diagnostics);
            program.generics.depth = depth;
        }
    }

    /// Checks the body of `nested` as `function`, the generic itself or one of its copies,
    /// with its type parameters standing for `arguments`. A copy's qualified name adds them,
    /// which only the name its code is linked under shows.
    fn check_nested_body(
        program: &mut Program,
        nested: &Pending,
        (function, arguments): (FunctionId, &[Type]),
        diagnostics: &mut Diagnostics,
    ) {
        let generic = nested.function;
        let saved = program.type_arguments.clone();
        program.type_arguments.extend(
            nested
                .definition
                .type_parameters
                .iter()
                .zip(arguments)
                .map(|(parameter, argument)| (parameter.name.clone(), argument.clone())),
        );
        let copy = function != generic;
        let qualified_name = if copy {
            let given: Vec<String> = arguments.iter().map(ToString::to_string).collect();
            format!("{}[{}]", nested.definition.qualified_name, given.join(", "))
        } else {
            nested.definition.qualified_name.clone()
        };
        let definition = Definition {
            name: nested.definition.name.clone(),
            qualified_name,
            class: None,
            takes_instance: false,
            copy_of: copy.then_some(generic),
            home: None,
            signature: program.instantiated(generic, arguments),
            type_parameters: Vec::new(),
            parameters: nested.definition.parameters.clone(),
            body: &nested.body,
            span: nested.definition.span,
        };
        Self::check_body_of(
            program,
            function,
            &definition,
            Some(nested.closure.clone()),
            diagnostics,
        );
        program.type_arguments = saved;
    }
}

/// Returns the next copy of one of `generic` the program asks for and has not checked, with
/// its nested function, after taking it from the copies that wait.
fn next_copy<'nested>(
    program: &mut Program,
    generic: &'nested [Pending],
) -> Option<(GenericCopy, &'nested Pending)> {
    let (position, nested) =
        program
            .generics
            .pending
            .iter()
            .enumerate()
            .find_map(|(position, index)| {
                let wanted = program.generics.copies[*index].generic;
                generic
                    .iter()
                    .find(|nested| nested.function == wanted)
                    .map(|nested| (position, nested))
            })?;
    let index = program.generics.pending.remove(position);
    Some((program.generics.copies[index].clone(), nested))
}
