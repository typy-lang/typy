//! The command line of `typy`, parsed with clap; clap prints the usage and exits with status 2
//! when it is wrong (CLI-06).

use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand};
use typy_build::{BuildMode, EmitStage, LibraryKind};
use typy_codegen::{Target, TargetTriple};
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
        mode: Mode,
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
        mode: Mode,
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

/// The build mode flags; at most one is given (CLI-05).
#[derive(Clone, Copy, Debug, Args)]
pub(crate) struct Mode {
    /// Optimise for speed.
    #[arg(long, conflicts_with = "release_small")]
    release: bool,
    /// Optimise for size.
    #[arg(long)]
    release_small: bool,
}

impl Mode {
    /// Returns the build mode the flags select; debug when neither is given.
    pub(crate) const fn build_mode(self) -> BuildMode {
        match (self.release, self.release_small) {
            (true, _) => BuildMode::Release,
            (false, true) => BuildMode::ReleaseSmall,
            (false, false) => BuildMode::Debug,
        }
    }
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
