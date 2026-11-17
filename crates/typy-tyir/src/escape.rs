//! Escape analysis: finds each instance that never outlives the function that makes it, which
//! the function then holds in its frame (MEM-03, ADR-0042).

mod aliases;
#[cfg(test)]
mod tests;

use crate::function::TyirFunction;
use crate::module::TyirModule;
use crate::operation::Operation;
use crate::value::{ValueId, ValueRepresentation};

use aliases::Aliases;

/// What the analysis of one function needs to know of the whole program.
pub(crate) struct Program<'module> {
    module: &'module TyirModule,
    /// For each function, by ID, whether each of its parameters escapes.
    parameters: Vec<Vec<bool>>,
    /// For each class, by index, the classes derived from it, itself first.
    derived: Vec<Vec<u32>>,
}

impl<'module> Program<'module> {
    fn new(module: &'module TyirModule) -> Self {
        let derived = (0..module.classes.len())
            .map(|class| {
                (0..module.classes.len())
                    .filter(|each| derives_from(module, *each, class))
                    .map(|each| u32::try_from(each).unwrap_or(u32::MAX))
                    .collect()
            })
            .collect();
        Self {
            module,
            parameters: module
                .functions
                .iter()
                .map(|function| vec![false; function.parameters.len()])
                .collect(),
            derived,
        }
    }

    /// Returns whether the parameter at `position` of `function` escapes; a position past its
    /// parameters is taken to.
    pub(crate) fn parameter_escapes(&self, function: crate::FunctionId, position: usize) -> bool {
        self.parameters
            .get(function.0 as usize)
            .and_then(|parameters| parameters.get(position))
            .copied()
            .unwrap_or(true)
    }

    /// Returns each function a call through position `slot` of the table of `class` may run:
    /// the one each table of the class and of the classes derived from it holds there.
    pub(crate) fn table_functions(&self, class: u32, slot: u32) -> Vec<crate::FunctionId> {
        self.derived
            .get(class as usize)
            .into_iter()
            .flatten()
            .filter_map(|each| {
                self.module.classes[*each as usize]
                    .methods
                    .get(slot as usize)
                    .copied()
            })
            .collect()
    }

    /// Adds each parameter of each function shown to escape, until no more is.
    fn find_escaping_parameters(&mut self) {
        loop {
            let mut changed = false;
            for (index, function) in self.module.functions.iter().enumerate() {
                let aliases = Aliases::of(self, function);
                for (position, parameter) in function.parameters.iter().enumerate() {
                    if !self.parameters[index][position] && aliases.escapes(*parameter) {
                        self.parameters[index][position] = true;
                        changed = true;
                    }
                }
            }
            if !changed {
                return;
            }
        }
    }

    /// Returns the instances of `function` that no path lets outlive it.
    fn frame_instances(&self, function: &TyirFunction) -> Vec<ValueId> {
        let aliases = Aliases::of(self, function);
        function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter_map(
                |instruction| match (instruction.result, &instruction.operation) {
                    (Some(result), Operation::NewInstance { class })
                        if may_be_in_frame(self.module, *class) && !aliases.escapes(result) =>
                    {
                        Some(result)
                    }
                    _ => None,
                },
            )
            .collect()
    }
}

/// Returns whether `class` is `base` or derives from it.
fn derives_from(module: &TyirModule, class: usize, base: usize) -> bool {
    let mut current = Some(class);
    // A chain of bases is no longer than the number of classes, so a cycle cannot hang this.
    for _ in 0..=module.classes.len() {
        match current {
            Some(each) if each == base => return true,
            Some(each) => {
                current = module
                    .classes
                    .get(each)
                    .and_then(|class| class.base)
                    .map(|base| base as usize);
            }
            None => return false,
        }
    }
    false
}

/// Returns whether an instance of `class` may be held in a frame: an instance of a class of
/// the program, not an exception, whose layout is the runtime's and not an instance's.
pub(crate) fn may_be_in_frame(module: &TyirModule, class: u32) -> bool {
    module
        .classes
        .get(class as usize)
        .is_some_and(|class| class.exception_base.is_none())
}

/// Marks each instance of a class of the program that no path lets outlive the function that
/// makes it as held in that function's frame (MEM-03, ADR-0042). A parameter escapes when
/// the function lets the value outlive the call, found for every function together.
pub fn place_instances(module: &mut TyirModule) {
    let mut program = Program::new(module);
    program.find_escaping_parameters();
    let placed: Vec<Vec<ValueId>> = module
        .functions
        .iter()
        .map(|function| program.frame_instances(function))
        .collect();
    for (function, values) in module.functions.iter_mut().zip(placed) {
        for value in values {
            function.values[value.0 as usize].representation = ValueRepresentation::Stack;
        }
    }
}
