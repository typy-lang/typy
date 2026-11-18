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
#[command(name = "typy", version, about)]
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

/// The commands of 0.1.
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
mod tests {
    use super::{Arguments, Command};
    use crate::output::ErrorFormat;
    use clap::Parser as _;
    use typy_build::{BuildMode, EmitStage};

    fn parse(line: &[&str]) -> Result<Command, clap::Error> {
        Arguments::try_parse_from(std::iter::once("typy").chain(line.iter().copied()))
            .map(|arguments| arguments.command)
    }

    #[test]
    fn build_takes_a_stage_an_output_and_a_mode() {
        let (file, emit, output, mode) = match parse(&[
            "build",
            "app.py",
            "--emit=tyir",
            "-o",
            "out.txt",
            "--release-small",
        ]) {
            Ok(Command::Build {
                file,
                emit,
                output,
                mode,
                ..
            }) => (file, emit, output, mode),
            other => panic!("not a build: {other:?}"),
        };
        assert_eq!((file.as_str(), emit), ("app.py", Some(EmitStage::Tyir)));
        assert_eq!(
            output.as_deref().map(camino::Utf8Path::as_str),
            Some("out.txt")
        );
        assert_eq!(mode.build_mode(), BuildMode::ReleaseSmall);
    }

    #[test]
    fn run_passes_the_arguments_after_two_dashes() {
        let (arguments, mode) = match parse(&["run", "app.py", "--release", "--", "-x", "two"]) {
            Ok(Command::Run {
                arguments, mode, ..
            }) => (arguments, mode),
            other => panic!("not a run: {other:?}"),
        };
        assert_eq!(arguments, ["-x", "two"]);
        assert_eq!(mode.build_mode(), BuildMode::Release);
    }

    #[test]
    fn wrong_usage_is_refused_with_status_2() {
        for line in [
            &["build"][..],
            &["build", "app.py", "--emit=exe"],
            &["build", "app.py", "--release", "--release-small"],
            &["check", "app.py", "--python-floor", "3.11"],
            &["frobnicate"],
        ] {
            let error = parse(line).unwrap_err();
            assert_eq!(error.exit_code(), 2, "{line:?}");
        }
    }

    #[test]
    fn every_command_takes_a_python_floor() {
        for line in [
            &["check", "app.py", "--python-floor", "3.15"][..],
            &["build", "app.py", "--python-floor=3.15"],
            &["run", "app.py", "--python-floor", "3.15", "--", "x"],
        ] {
            let settings = match parse(line).unwrap() {
                Command::Check { settings, .. }
                | Command::Build { settings, .. }
                | Command::Run { settings, .. } => settings,
                Command::Explain { .. } => panic!("explain takes no settings"),
            };
            assert_eq!(
                settings.overrides().python_floor,
                Some(typy_source::PythonFloor::V3_15)
            );
        }
    }

    #[test]
    fn the_error_format_is_text_unless_json_is_asked_for() {
        let format = |line: &[&str]| {
            Arguments::try_parse_from(std::iter::once("typy").chain(line.iter().copied()))
                .map(|arguments| arguments.error_format)
        };
        assert_eq!(format(&["check", "a.py"]).unwrap(), ErrorFormat::Text);
        assert_eq!(
            format(&["check", "a.py", "--error-format=json"]).unwrap(),
            ErrorFormat::Json
        );
        assert_eq!(
            format(&["--error-format", "json", "run", "a.py"]).unwrap(),
            ErrorFormat::Json
        );
        assert!(format(&["check", "a.py", "--error-format=xml"]).is_err());
        let deny = Arguments::try_parse_from(["typy", "build", "a.py", "--deny-warnings"]);
        assert!(deny.unwrap().deny_warnings);
    }

    #[test]
    fn check_takes_a_file() {
        assert!(matches!(
            parse(&["check", "app.py"]),
            Ok(Command::Check { .. })
        ));
        assert_eq!(
            parse(&["build", "a.py"])
                .map(|_| ())
                .err()
                .map(|error| error.kind()),
            None
        );
    }
}
