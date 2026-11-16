//! The copies of generic functions and of the methods of generic classes that calls need,
//! each checked as an ordinary function in the names of the module that defines its generic
//! (GEN-03, GEN-07, ADR-0023).

use typy_diagnostics::Diagnostics;

use super::{Definition, ModuleDefinitions, check_body};
use crate::body::Program;

/// Checks each copy made and not yet checked, whose type parameters stand for its arguments;
/// a copy may need further copies. `own` holds the definitions of the module being checked,
/// with the ID of the first, and `others` those of the modules checked before it.
pub(super) fn check(
    program: &mut Program,
    own: (&[Definition<'_>], usize),
    others: &ModuleDefinitions<'_>,
    diagnostics: &mut Diagnostics,
) {
    while let Some(copy) = program.next_pending_copy() {
        let Some(home) = program
            .generic(copy.generic)
            .map(|generic| generic.module.clone())
        else {
            continue;
        };
        let (definitions, first_function) = if home == program.module_name() {
            own
        } else if let Some((definitions, first_function)) = others.get(&home) {
            (definitions.as_slice(), *first_function)
        } else {
            continue;
        };
        let generic = &definitions[copy.generic.0 as usize - first_function];
        let given: Vec<String> = copy.arguments.iter().map(ToString::to_string).collect();
        // A copy of a method belongs to its copy of the class, whose name it takes.
        let qualified_name = match copy.class {
            Some(class) => format!(
                "{}[{}].{}",
                program.class_name(class),
                given.join(", "),
                generic.name
            ),
            None => format!("{}[{}]", generic.qualified_name, given.join(", ")),
        };
        let made = Definition {
            qualified_name,
            class: copy.class.or(generic.class),
            copy_of: Some(copy.generic),
            signature: program.instantiated(copy.generic, &copy.arguments),
            type_parameters: Vec::new(),
            ..generic.clone()
        };
        program.type_arguments = generic
            .type_parameters
            .iter()
            .zip(&copy.arguments)
            .map(|(parameter, argument)| (parameter.name.clone(), argument.clone()))
            .collect();
        // The body of a generic of another module reads that module's names.
        let elsewhere = (home != program.module_name())
            .then(|| program.context_of(&home))
            .flatten()
            .map(|context| program.swap_module_context(context));
        check_body(program, copy.copy, &made, diagnostics);
        if let Some(current) = elsewhere {
            program.swap_module_context(current);
        }
        program.type_arguments.clear();
    }
    program.end_copies();
}
