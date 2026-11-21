//! The check of a generic function's body: once with each type parameter opaque, and once
//! for each combination of the constraints of its constrained parameters. The body itself is
//! never run; its copies are (GEN-01, GEN-02, ADR-0023).

use typy_diagnostics::Diagnostics;

use super::Definition;
use crate::body::{BodyChecker, Program};
use crate::typed::FunctionId;
use crate::types::Type;

/// Checks the body of the generic function `id`, then keeps it as a generic, without a body.
pub(super) fn check(
    program: &mut Program,
    id: FunctionId,
    definition: &Definition<'_>,
    diagnostics: &mut Diagnostics,
) {
    program.generic_pass = Some(id);
    for arguments in combinations(definition) {
        program.type_arguments = definition
            .type_parameters
            .iter()
            .zip(&arguments)
            .map(|(parameter, argument)| (parameter.name.clone(), argument.clone()))
            .collect();
        let checked = Definition {
            signature: program.instantiated(id, &arguments),
            type_parameters: Vec::new(),
            ..definition.clone()
        };
        BodyChecker::check_function(program, id, &checked, diagnostics);
    }
    program.type_arguments.clear();
    program.generic_pass = None;
}

/// Returns each list of type arguments the body is checked with: each unconstrained
/// parameter as its opaque type, and each constrained one as each of its constraints.
fn combinations(definition: &Definition<'_>) -> Vec<Vec<Type>> {
    let mut lists = vec![Vec::new()];
    for parameter in &definition.type_parameters {
        let choices = if parameter.constraints.is_empty() {
            vec![Type::parameter(parameter.clone())]
        } else {
            parameter.constraints.clone()
        };
        lists = lists
            .into_iter()
            .flat_map(|list: Vec<Type>| {
                choices.iter().map(move |choice| {
                    let mut longer = list.clone();
                    longer.push(choice.clone());
                    longer
                })
            })
            .collect();
    }
    lists
}
