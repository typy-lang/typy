//! `typy print WHAT`: what `typy` would use, one fact a line, for a person or a tool (CLI-25).

use std::fmt::Write as _;

use camino::{Utf8Path, Utf8PathBuf};
use typy_build::{BuildMode, RuntimeArchive};
use typy_codegen::{Target, TargetTriple};
use typy_driver::{Failure, Sources};
use typy_source::{LibraryLink, ProjectSettings, PythonFloor, SettingOverrides};

use crate::arguments::{PrintRequest, TargetFlag};
use crate::exit_status::ExitStatus;
use crate::output::Reporter;

/// Writes what `what` asks for to standard output.
pub(super) fn print(
    what: PrintRequest,
    file: Option<&Utf8Path>,
    overrides: SettingOverrides,
    triple: Option<TargetTriple>,
    reporter: Reporter,
) -> ExitStatus {
    let host = Target::host().triple;
    let triple = triple.unwrap_or(host);
    let text = match what {
        PrintRequest::Targets => targets(host),
        PrintRequest::Host => format!("{host}\n"),
        PrintRequest::TargetCpus => match processors(triple, host) {
            Ok(text) => text,
            Err(message) => {
                reporter.internal(&message);
                return ExitStatus::Internal;
            }
        },
        PrintRequest::Paths => match super::compiler() {
            Ok(compiler) => paths(&compiler, triple),
            Err(message) => {
                reporter.internal(&message);
                return ExitStatus::Internal;
            }
        },
        PrintRequest::Settings => {
            let root = file
                .and_then(Utf8Path::parent)
                .filter(|parent| !parent.as_str().is_empty())
                .map_or_else(|| Utf8PathBuf::from("."), Utf8Path::to_owned);
            match ProjectSettings::read(&root, overrides) {
                Ok(settings) => settings_text(&settings),
                Err(failure) => {
                    return reporter.report(Failure {
                        diagnostics: failure.diagnostics.clone(),
                        sources: Sources::Unloaded(Box::new(failure)),
                    });
                }
            }
        }
        PrintRequest::Version => version(),
    };
    print!("{text}");
    ExitStatus::Success
}

/// Writes LLVM's list of a target's processors to standard error, for the parent `typy` that
/// reads it.
pub(super) fn processor_help(target: TargetFlag, reporter: Reporter) -> ExitStatus {
    let triple = target.triple.unwrap_or(Target::host().triple);
    match typy_codegen::write_processor_help(triple) {
        Ok(()) => ExitStatus::Success,
        Err(diagnostics) => reporter.report(Failure::without_sources(diagnostics)),
    }
}

/// Returns each supported target, saying which this host builds for.
fn targets(host: TargetTriple) -> String {
    TargetTriple::ALL
        .iter()
        .fold(String::new(), |mut text, triple| {
            let note = if *triple == host {
                " (host)"
            } else if triple.builds_on(host) {
                ""
            } else {
                " (built on another host)"
            };
            let _ = writeln!(text, "{triple}{note}");
            text
        })
}

/// Returns the processors of `triple`: its baseline, which builds use by default, `native`
/// when this host runs the target, and every processor LLVM knows for it.
fn processors(triple: TargetTriple, host: TargetTriple) -> Result<String, String> {
    let mut text = format!("{} (baseline, the default)\n", triple.cpu());
    if triple == host {
        let _ = writeln!(
            text,
            "native (this machine: {})",
            typy_codegen::host_processor()
        );
    }
    for name in known_processors(triple)? {
        let _ = writeln!(text, "{name}");
    }
    Ok(text)
}

/// Returns the processors LLVM knows for `triple`, which it lists only on standard error, so
/// a child `typy` lists them and this one reads them.
pub(super) fn known_processors(triple: TargetTriple) -> Result<Vec<String>, String> {
    let compiler = super::compiler()?;
    let output = std::process::Command::new(compiler.as_std_path())
        .args(["llvm-processor-help", "--target", triple.as_str()])
        .output()
        .map_err(|error| format!("cannot run {compiler}: {error}"))?;
    let listed = String::from_utf8_lossy(&output.stderr);
    Ok(processor_names(&listed)
        .into_iter()
        .map(str::to_owned)
        .collect())
}

/// Returns the processor names of LLVM's help text: the first word of each line between
/// "Available CPUs for this target:" and the list of features.
fn processor_names(listed: &str) -> Vec<&str> {
    listed
        .lines()
        .skip_while(|line| !line.starts_with("Available CPUs"))
        .skip(1)
        .take_while(|line| !line.starts_with("Available features"))
        .filter_map(|line| line.split_whitespace().next())
        .collect()
}

/// Returns the compiler's path, the runtime archives it links for `triple` and its cache.
fn paths(compiler: &Utf8Path, triple: TargetTriple) -> String {
    let speed = RuntimeArchive::path_beside(compiler, triple, BuildMode::RELEASE);
    let size = RuntimeArchive::path_beside(compiler, triple, BuildMode::RELEASE_SMALL);
    format!(
        "compiler = {}\nruntime = {}\nruntime-small = {}\ncache = {}\n",
        toml_string(compiler.as_str()),
        toml_string(speed.as_str()),
        toml_string(size.as_str()),
        toml_string(crate::cache::directory().as_str()),
    )
}

/// Returns the settings in `typy.toml`'s own keys, the command line's overrides applied.
fn settings_text(settings: &ProjectSettings) -> String {
    let list = |paths: &[Utf8PathBuf]| {
        let items: Vec<String> = paths
            .iter()
            .map(|path| toml_string(path.as_str()))
            .collect();
        format!("[{}]", items.join(", "))
    };
    let mut text = format!(
        "python-floor = {}\nmodule-paths = {}\nlibrary-paths = {}\n",
        toml_string(settings.python_floor.as_str()),
        list(&settings.module_paths),
        list(&settings.library_paths),
    );
    if let Some(version) = &settings.version {
        let _ = writeln!(text, "version = {}", toml_string(version));
    }
    if let Some(allocator) = &settings.allocator {
        for (key, value) in [
            ("allocate", Some(&allocator.allocate)),
            ("reallocate", Some(&allocator.reallocate)),
            ("free", Some(&allocator.free)),
            ("library", allocator.library.as_ref()),
        ] {
            if let Some(value) = value {
                let _ = writeln!(text, "allocator.{key} = {}", toml_string(value));
            }
        }
    }
    for (name, link) in &settings.libraries {
        let link = match link {
            LibraryLink::Static => "static",
            LibraryLink::Dynamic => "dynamic",
        };
        let _ = writeln!(
            text,
            "libraries.{}.link = {}",
            toml_key(name),
            toml_string(link)
        );
    }
    for (triple, tools) in &settings.targets {
        for (key, value) in [
            ("linker", &tools.linker),
            ("ar", &tools.archiver),
            ("objcopy", &tools.object_copier),
        ] {
            if let Some(value) = value {
                let _ = writeln!(
                    text,
                    "target.{}.{key} = {}",
                    toml_key(triple),
                    toml_string(value)
                );
            }
        }
    }
    text
}

/// Returns the versions of `typy`, of the runtime it links, of LLVM, and the Python floors.
fn version() -> String {
    let (major, minor, patch) = typy_codegen::llvm_version();
    let floors: Vec<&str> = PythonFloor::ALL
        .iter()
        .map(|floor| floor.as_str())
        .collect();
    format!(
        "typy = {}\nruntime = {}\nllvm = \"{major}.{minor}.{patch}\"\npython-floors = [{}]\n",
        toml_string(env!("CARGO_PKG_VERSION")),
        toml_string(env!("CARGO_PKG_VERSION")),
        floors
            .iter()
            .map(|floor| toml_string(floor))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

/// Returns `text` as a TOML basic string.
fn toml_string(text: &str) -> String {
    let mut quoted = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\t' => quoted.push_str("\\t"),
            control if control.is_control() => {
                let _ = write!(quoted, "\\u{:04X}", u32::from(control));
            }
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

/// Returns `key` bare when TOML allows it, and quoted otherwise.
fn toml_key(key: &str) -> String {
    if !key.is_empty()
        && key
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        key.to_owned()
    } else {
        toml_string(key)
    }
}

#[cfg(test)]
mod tests;
