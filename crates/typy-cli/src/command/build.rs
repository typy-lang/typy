//! `typy build FILE` (CLI-01, CLI-04, CLI-05, CLI-09, UC-02, UC-05).

use camino::Utf8PathBuf;
use typy_build::{BuildMode, BuildRequest, EmitStage, LibraryKind};
use typy_codegen::{Target, TargetTriple};
use typy_driver::{Outcome, Pipeline};

use crate::exit_status::ExitStatus;
use crate::output::Reporter;

/// What the flags of `typy build` ask for.
pub(super) struct Flags {
    /// The stage to write instead of an executable.
    pub(super) emit: Option<EmitStage>,
    /// Where to write the executable, the library or the stage.
    pub(super) output: Option<Utf8PathBuf>,
    /// The build mode.
    pub(super) mode: BuildMode,
    /// The kind of library to build instead of an executable.
    pub(super) library: Option<LibraryKind>,
    /// The target, the host's when absent (CLI-08).
    pub(super) target: Option<TargetTriple>,
}

/// Builds the executable, or the library of the flags' kind, or writes the output of the
/// stage they name.
pub(super) fn build(
    pipeline: &Pipeline,
    file: Utf8PathBuf,
    flags: &Flags,
    reporter: Reporter,
) -> ExitStatus {
    let mut request = match BuildRequest::new(file) {
        Ok(request) => request.with_mode(flags.mode),
        Err(error) => return super::usage(&error),
    };
    if let Some(triple) = flags.target {
        request = request.with_target(Target { triple });
    }
    if let Some(stage) = flags.emit {
        request = request.with_emit(stage);
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
    match success.outcome {
        Outcome::Emitted(emission) => reporter.emission(&emission, flags.output.as_deref()),
        Outcome::Executable(_) | Outcome::Checked(_) => ExitStatus::Success,
    }
}
