//! `typy build FILE` (CLI-01, CLI-04, CLI-05, CLI-09, CLI-29, UC-02, UC-05).

use camino::Utf8PathBuf;
use typy_build::{BuildRequest, EmitStage, LibraryKind};
use typy_codegen::{Target, TargetTriple};
use typy_driver::{Outcome, Pipeline};

use crate::exit_status::ExitStatus;
use crate::output::Reporter;

/// What the flags of `typy build` ask for.
pub(super) struct Flags {
    /// The stages to write instead of an executable.
    pub(super) emit: Vec<EmitStage>,
    /// Where to write the executable, the library or the stage.
    pub(super) output: Option<Utf8PathBuf>,
    /// How the build generates code: its mode, processor and features.
    pub(super) generation: super::generation::Generation,
    /// The kind of library to build instead of an executable.
    pub(super) library: Option<LibraryKind>,
    /// The target, the host's when absent (CLI-08).
    pub(super) target: Option<TargetTriple>,
}

/// Builds the executable, or the library of the flags' kind, or writes the output of the
/// stages they name: one to `-o` or standard output, several each beside the output (CLI-29).
pub(super) fn build(
    pipeline: &Pipeline,
    file: Utf8PathBuf,
    flags: &Flags,
    reporter: Reporter,
) -> ExitStatus {
    let mut request = match BuildRequest::new(file) {
        Ok(request) => request.with_mode(flags.generation.mode).with_processor(
            flags.generation.processor.clone(),
            flags.generation.features.clone(),
        ),
        Err(error) => return super::usage(&error),
    };
    if let Some(triple) = flags.target {
        request = request.with_target(Target { triple });
    }
    for stage in &flags.emit {
        request = request.with_emit(*stage);
    }
    if let Some(kind) = flags.library {
        request = request.with_library(kind);
    }
    if let Some(path) = &flags.output {
        request = request.with_output(path);
    }
    let success = match pipeline.run(&request) {
        Ok(success) => success,
        Err(failure) => return reporter.report(failure),
    };
    reporter.warnings(success.warnings);
    let Outcome::Emitted(emitted) = success.outcome else {
        return ExitStatus::Success;
    };
    if let [(_, emission)] = emitted.as_slice() {
        return reporter.emission(emission, flags.output.as_deref());
    }
    let output = request.output_path();
    for (stage, emission) in &emitted {
        let status = reporter.emission(emission, Some(&stage.path_beside(&output)));
        if status != ExitStatus::Success {
            return status;
        }
    }
    ExitStatus::Success
}
