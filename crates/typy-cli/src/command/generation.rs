//! Resolves what the code generation flags choose: `native` to this machine's processor and
//! features, and any other processor checked against LLVM's list for the target (CLI-20,
//! CLI-21, CLI-30).

use typy_build::BuildMode;
use typy_codegen::{Target, TargetTriple};

use crate::arguments::Choice;

/// How a build generates code.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Generation {
    /// The build mode, from the optimisation level.
    pub(super) mode: BuildMode,
    /// The processor, `None` for the target's baseline.
    pub(super) processor: Option<String>,
    /// The features beside the processor's, as LLVM writes them.
    pub(super) features: String,
}

/// Returns how a build for `triple` generates code; a processor that is not the host's for
/// `native`, or that LLVM does not know for the target, is wrong usage.
pub(super) fn resolve(choice: Choice, triple: TargetTriple) -> Result<Generation, String> {
    let host = Target::host().triple;
    let (processor, features) = match choice.processor.as_deref() {
        None => (None, choice.features),
        Some("native") => {
            if triple != host {
                return Err(format!(
                    "`native` is the processor of this machine, which builds for {host}, not \
                     {triple}"
                ));
            }
            (
                Some(typy_codegen::host_processor()),
                joined(&typy_codegen::host_features(), &choice.features),
            )
        }
        Some(name) => {
            if name != triple.cpu()
                && !super::print::known_processors(triple)?
                    .iter()
                    .any(|known| known == name)
            {
                return Err(format!(
                    "`{name}` is not a processor of {triple}; `typy print target-cpus --target \
                     {triple}` lists them"
                ));
            }
            (Some(name.to_owned()), choice.features)
        }
    };
    Ok(Generation {
        mode: choice.mode,
        processor,
        features,
    })
}

/// Returns two feature lists as one, the second's features after, so they win.
fn joined(first: &str, second: &str) -> String {
    match (first.is_empty(), second.is_empty()) {
        (_, true) => first.to_owned(),
        (true, false) => second.to_owned(),
        (false, false) => format!("{first},{second}"),
    }
}

#[cfg(test)]
mod tests;
