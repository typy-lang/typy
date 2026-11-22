//! The command line of `typy`, parsed with clap; clap prints the usage and exits with status 2
//! when it is wrong (CLI-06).

use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand};
use typy_build::{BuildMode, EmitStage, LibraryKind};
use typy_codegen::{Optimization, Target, TargetTriple};
use typy_source::{PythonFloor, SettingOverrides};

use crate::output::ErrorFormat;

/// Compiles typed Python programs ahead of time to native executables.
#[derive(Debug, Parser)]
#[command(name = "typy", version, about, allow_external_subcommands = true)]
pub(crate) struct Arguments {
    #[command(subcommand)]
    pub(crate) command: Command,
    /// How to write diagnostics: text, or one JSON object per line.
    #[arg(
        long,
        global = true,
        value_enum,
        default_value_t,
        value_name = "FORMAT"
    )]
    pub(crate) error_format: ErrorFormat,
    /// Fail with status 1 when there are warnings, before any code is generated.
    #[arg(long, global = true)]
    pub(crate) deny_warnings: bool,
    /// Write each command typy runs, such as the C linker with its arguments, to standard
    /// error before running it.
    #[arg(short, long, global = true)]
    pub(crate) verbose: bool,
    /// Write how long each stage of the command took to standard error.
    #[arg(long, global = true)]
    pub(crate) timings: bool,
}

/// The options every command takes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Global {
    /// `--deny-warnings` (DIA-07).
    pub(crate) deny_warnings: bool,
    /// `-v` (CLI-23).
    pub(crate) verbose: bool,
    /// `--timings` (CLI-22).
    pub(crate) timings: bool,
}

impl Arguments {
    /// Returns the options every command takes.
    pub(crate) const fn global(&self) -> Global {
        Global {
            deny_warnings: self.deny_warnings,
            verbose: self.verbose,
            timings: self.timings,
        }
    }
}

/// The commands of `typy`; any other runs the program `typy-NAME` from `PATH` (ADR-0105).
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Report every diagnostic without generating code.
    Check {
        /// The main file of the program.
        file: Utf8PathBuf,
        #[command(flatten)]
        settings: Settings,
        #[command(flatten)]
        library: Library,
        #[command(flatten)]
        target: TargetFlag,
    },
    /// Build an executable, or write the output of one compiler stage.
    Build {
        /// The main file of the program.
        file: Utf8PathBuf,
        /// Write this stage instead of an executable: ast, typed-ast, tyir, tyir-mem,
        /// llvm-ir, asm or obj.
        #[arg(long, value_name = "STAGE", value_parser = parse_stage)]
        emit: Option<EmitStage>,
        /// Where to write the executable or the stage; standard output for a stage when absent.
        #[arg(short, long, value_name = "PATH")]
        output: Option<Utf8PathBuf>,
        #[command(flatten)]
        generation: Generation,
        #[command(flatten)]
        temps: Temps,
        #[command(flatten)]
        settings: Settings,
        #[command(flatten)]
        library: Library,
        #[command(flatten)]
        target: TargetFlag,
    },
    /// Explain a diagnostic code, with a program that causes it and the corrected program.
    Explain {
        /// The code, such as E0402.
        code: String,
    },
    /// Build the program and run it, passing the arguments after `--`.
    Run {
        /// The main file of the program.
        file: Utf8PathBuf,
        #[command(flatten)]
        generation: Generation,
        #[command(flatten)]
        temps: Temps,
        #[command(flatten)]
        settings: Settings,
        /// The program's arguments.
        #[arg(last = true)]
        arguments: Vec<String>,
    },
    /// Write what typy would use: its targets, a target's processors, the host, its paths, a
    /// project's settings, or its version.
    Print {
        /// What to write.
        #[arg(value_enum)]
        what: PrintRequest,
        /// The main file of the program whose project `settings` reads; the current directory's
        /// project when absent.
        file: Option<Utf8PathBuf>,
        #[command(flatten)]
        settings: Settings,
        #[command(flatten)]
        library: Library,
        #[command(flatten)]
        target: TargetFlag,
    },
    /// Writes LLVM's list of a target's processors to standard error, which `print
    /// target-cpus` reads from a child `typy`, as LLVM writes it only there.
    #[command(name = "llvm-processor-help", hide = true)]
    ProcessorHelp {
        #[command(flatten)]
        target: TargetFlag,
    },
    /// Any other command, which runs the program `typy-NAME` found on `PATH` (CLI-32).
    #[command(external_subcommand)]
    External(Vec<String>),
}

/// What `typy print` writes (CLI-25).
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum PrintRequest {
    /// Every supported target, and whether this host builds for it.
    Targets,
    /// The processors of a target: its baseline, `native`, and those LLVM knows.
    TargetCpus,
    /// The target `typy` runs on.
    Host,
    /// The compiler, its runtime archives and its cache.
    Paths,
    /// The settings a command would use for a project.
    Settings,
    /// The versions of `typy`, its runtime, LLVM and the Python floors.
    Version,
}

/// The flags that choose how code is generated: the optimisation level, the processor and its
/// features (CLI-05, CLI-20, CLI-21, CLI-30, ADR-0104).
#[derive(Clone, Debug, Default, Args)]
pub(crate) struct Generation {
    /// Optimise for speed: `-O2`.
    #[arg(long, conflicts_with = "release_small")]
    release: bool,
    /// Optimise for size: `-Oz`.
    #[arg(long)]
    release_small: bool,
    /// The optimisation level: 0, 1, 2 or 3, s for small code, or z for the smallest.
    #[arg(short = 'O', value_name = "LEVEL", value_parser = parse_level)]
    level: Option<Optimization>,
    /// The processor to generate code for: `native` for this machine, or one that `typy
    /// print target-cpus` lists; the target's baseline, which every machine of it runs, when
    /// absent.
    #[arg(long, value_name = "NAME")]
    target_cpu: Option<String>,
    /// A code generation option, KEY=VALUE: opt-level, target-cpu or target-feature; `typy -C
    /// help` lists them.
    #[arg(short = 'C', value_name = "KEY=VALUE", value_parser = parse_codegen_option)]
    options: Vec<CodegenOption>,
}

/// A code generation option given with `-C` (CLI-30).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CodegenOption {
    /// `opt-level=LEVEL`, as `-O` gives it.
    OptLevel(Optimization),
    /// `target-cpu=NAME`, as `--target-cpu` gives it.
    TargetCpu(String),
    /// `target-feature=LIST`, features to turn on or off beside the processor's.
    TargetFeature(String),
}

/// The keys `-C` takes, each with what its value is, as `typy -C help` lists them.
pub(crate) const CODEGEN_KEYS: [(&str, &str); 3] = [
    (
        "opt-level=LEVEL",
        "the optimisation level, 0, 1, 2, 3, s or z, as -O gives it",
    ),
    (
        "target-cpu=NAME",
        "the processor, as --target-cpu gives it; `typy print target-cpus` lists them",
    ),
    (
        "target-feature=LIST",
        "features to turn on or off beside the processor's, such as +avx2,-sse4a",
    ),
];

/// What the code generation flags choose, before a named processor is checked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Choice {
    /// The build mode, from the optimisation level.
    pub(crate) mode: BuildMode,
    /// The processor named, `native` among them, or `None` for the baseline.
    pub(crate) processor: Option<String>,
    /// The features `-C target-feature` gives, joined by commas.
    pub(crate) features: String,
}

impl Generation {
    /// Returns what the flags choose; a level or a processor given twice with two values is
    /// wrong usage, as a flag and its `-C` key are one setting (ADR-0104).
    pub(crate) fn choice(&self) -> Result<Choice, String> {
        let mut levels: Vec<(String, Optimization)> = Vec::new();
        if self.release {
            levels.push(("--release".to_owned(), Optimization::Speed));
        }
        if self.release_small {
            levels.push(("--release-small".to_owned(), Optimization::Size));
        }
        if let Some(level) = self.level {
            levels.push((format!("-O{}", level.letter()), level));
        }
        let mut processors: Vec<(String, String)> = self
            .target_cpu
            .iter()
            .map(|name| (format!("--target-cpu {name}"), name.clone()))
            .collect();
        let mut features: Vec<&str> = Vec::new();
        for option in &self.options {
            match option {
                CodegenOption::OptLevel(level) => {
                    levels.push((format!("-C opt-level={}", level.letter()), *level));
                }
                CodegenOption::TargetCpu(name) => {
                    processors.push((format!("-C target-cpu={name}"), name.clone()));
                }
                CodegenOption::TargetFeature(list) => features.push(list),
            }
        }
        let level = one(&levels, "optimisation level")?.unwrap_or_default();
        Ok(Choice {
            mode: BuildMode::of(level),
            processor: one(&processors, "processor")?,
            features: features.join(","),
        })
    }
}

/// Returns the one value the flags in `given` set, or `None` when none does; two flags that
/// set different values are wrong usage.
fn one<T: Clone + PartialEq>(given: &[(String, T)], what: &str) -> Result<Option<T>, String> {
    let Some((first_flag, first)) = given.first() else {
        return Ok(None);
    };
    if let Some((flag, _)) = given.iter().find(|(_, value)| value != first) {
        return Err(format!(
            "{first_flag} and {flag} give two {what}s; give one"
        ));
    }
    Ok(Some(first.clone()))
}

fn parse_level(text: &str) -> Result<Optimization, String> {
    Optimization::parse(text)
        .ok_or_else(|| format!("`{text}` is not a level; use 0, 1, 2, 3, s or z"))
}

fn parse_codegen_option(text: &str) -> Result<CodegenOption, String> {
    let (key, value) = text
        .split_once('=')
        .ok_or_else(|| format!("`{text}` is not KEY=VALUE; `typy -C help` lists the keys"))?;
    match key {
        "opt-level" => parse_level(value).map(CodegenOption::OptLevel),
        "target-cpu" if !value.is_empty() => Ok(CodegenOption::TargetCpu(value.to_owned())),
        "target-feature" if !value.is_empty() => Ok(CodegenOption::TargetFeature(value.to_owned())),
        "target-cpu" | "target-feature" => Err(format!("`{key}` needs a value")),
        _ => Err(format!(
            "`{key}` is not a code generation option; `typy -C help` lists them"
        )),
    }
}

/// The flag that keeps a build's intermediate files (CLI-24).
#[derive(Clone, Copy, Debug, Args)]
pub(crate) struct Temps {
    /// Keep the build's object file beside its output, named after it: `app.o` for `app`.
    #[arg(long)]
    pub(crate) save_temps: bool,
}

/// The flag that builds or checks the main file as a C library (CLI-09).
#[derive(Clone, Copy, Debug, Args)]
pub(crate) struct Library {
    /// Build a C library instead of an executable, static or shared: the module's `__all__`
    /// names the functions it exports.
    #[arg(long = "lib", value_name = "KIND", value_parser = parse_library)]
    pub(crate) kind: Option<LibraryKind>,
}

/// The flag that names the target to build or check for (CLI-08).
#[derive(Clone, Copy, Debug, Args)]
pub(crate) struct TargetFlag {
    /// Build for this target instead of the host's own. A Linux host builds for
    /// x86_64-unknown-linux-gnu, x86_64-unknown-linux-musl and aarch64-unknown-linux-gnu, and
    /// a macOS host for aarch64-apple-darwin.
    #[arg(long = "target", value_name = "TRIPLE", value_parser = parse_target)]
    pub(crate) triple: Option<TargetTriple>,
}

/// The flags that override a setting of `typy.toml` (CLI-07).
#[derive(Clone, Copy, Debug, Args)]
pub(crate) struct Settings {
    /// The oldest CPython version the program supports, from 3.12 to 3.15; overrides
    /// `python-floor` in typy.toml.
    #[arg(long, value_name = "VERSION", value_parser = parse_floor)]
    python_floor: Option<PythonFloor>,
}

impl Settings {
    /// Returns the settings the flags give.
    pub(crate) const fn overrides(self) -> SettingOverrides {
        SettingOverrides {
            python_floor: self.python_floor,
            library: false,
        }
    }

    /// Returns the settings the flags give for a build or a check of a library of `kind`.
    pub(crate) const fn overrides_for(self, library: Library) -> SettingOverrides {
        let mut overrides = self.overrides();
        overrides.library = library.kind.is_some();
        overrides
    }
}

fn parse_floor(text: &str) -> Result<PythonFloor, String> {
    PythonFloor::parse(text).ok_or_else(|| {
        let floors: Vec<&str> = PythonFloor::ALL
            .iter()
            .map(|floor| floor.as_str())
            .collect();
        format!("expected one of {}", floors.join(", "))
    })
}

/// Parses a triple the host builds for; another is refused with the host's triples (CLI-08).
fn parse_target(text: &str) -> Result<TargetTriple, String> {
    let host = Target::host().triple;
    let triples: Vec<&str> = TargetTriple::ALL
        .iter()
        .filter(|triple| triple.builds_on(host))
        .map(|triple| triple.as_str())
        .collect();
    match TargetTriple::parse(text) {
        Some(triple) if triple.builds_on(host) => Ok(triple),
        Some(triple) => {
            let system = if triple.is_macos() { "macOS" } else { "Linux" };
            Err(format!(
                "{triple} is built on {system} only; expected one of {}",
                triples.join(", ")
            ))
        }
        None => Err(format!("expected one of {}", triples.join(", "))),
    }
}

fn parse_library(text: &str) -> Result<LibraryKind, String> {
    LibraryKind::parse(text).ok_or_else(|| {
        let kinds: Vec<&str> = LibraryKind::ALL.iter().map(|kind| kind.name()).collect();
        format!("expected one of {}", kinds.join(", "))
    })
}

fn parse_stage(text: &str) -> Result<EmitStage, String> {
    EmitStage::parse(text).ok_or_else(|| {
        let names: Vec<&str> = EmitStage::ALL.iter().map(|stage| stage.name()).collect();
        format!("expected one of {}", names.join(", "))
    })
}

#[cfg(test)]
mod tests;
