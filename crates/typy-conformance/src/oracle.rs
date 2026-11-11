//! Runs a test under each CPython oracle from its Python floor up (A-02, ADR-0008).

use std::process::Command;

use camino::Utf8Path;

use typy_source::PythonFloor;

use crate::observation::Observation;

/// Returns what each oracle version from `floor` up showed, with the version it ran; a run that
/// cannot be observed gives its reason.
pub(crate) fn run_oracles(
    file: &Utf8Path,
    floor: PythonFloor,
) -> Vec<(PythonFloor, Result<Observation, String>)> {
    PythonFloor::ALL
        .into_iter()
        .filter(|version| version.minor() >= floor.minor())
        .map(|version| {
            let program = format!("python{}", version.as_str());
            let mut command = Command::new(program);
            // Ignoring the environment and the user's site packages, without the isolated
            // mode's `-P`, keeps the test's directory first on the module path, so a test of
            // several modules imports its own; `-B` writes no bytecode beside them, and UTF-8
            // mode makes the output encoding the same on every machine.
            command.args(["-E", "-s", "-B", "-X", "utf8", file.as_str()]);
            let directory = file.parent().map(Utf8Path::to_owned);
            if let Some(directory) = &directory {
                command.current_dir(directory);
            }
            // CPython names a file by its resolved absolute path.
            let resolved = directory
                .as_ref()
                .and_then(|directory| std::fs::canonicalize(directory).ok());
            let observed = Observation::of(&mut command).map(|observation| match &resolved {
                Some(directory) => observation.without_prefix(&format!("{}/", directory.display())),
                None => observation,
            });
            (version, observed)
        })
        .collect()
}
