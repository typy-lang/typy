//! `typy build FILE` (CLI-01, CLI-04, CLI-05, UC-02, UC-05).

use camino::{Utf8Path, Utf8PathBuf};
use typy_build::{BuildMode, BuildRequest, EmitStage};
use typy_driver::{Outcome, Pipeline};

use crate::exit_status::ExitStatus;
use crate::output::Reporter;

/// Builds the executable, or writes the output of the stage `emit` names.
pub(super) fn build(
    pipeline: &Pipeline,
    file: Utf8PathBuf,
    emit: Option<EmitStage>,
    output: Option<&Utf8Path>,
    mode: BuildMode,
    reporter: Reporter,
) -> ExitStatus {
    let mut request = match BuildRequest::new(file) {
        Ok(request) => request.with_mode(mode),
        Err(error) => return super::usage(&error),
    };
    if let Some(stage) = emit {
        request = request.with_emit(stage);
    }
    if let Some(path) = output {
        request = request.with_output(path);
    }
    match pipeline.run(&request) {
        Ok(Outcome::Emitted(emission)) => reporter.emission(&emission, output),
        Ok(Outcome::Executable(_) | Outcome::Checked(_)) => ExitStatus::Success,
        Err(failure) => reporter.report(failure),
    }
}
