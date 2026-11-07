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
            // Isolated mode ignores the environment and the user's site packages, and UTF-8
            // mode makes the output encoding the same on every machine.
            command.args(["-I", "-X", "utf8", file.as_str()]);
            if let Some(directory) = file.parent() {
                command.current_dir(directory);
            }
            (version, Observation::of(&mut command))
        })
        .collect()
}
