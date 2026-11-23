//! Tests of `arguments`.

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
    let (file, emit, output, generation) = match parse(&[
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
            generation,
            ..
        }) => (file, emit, output, generation),
        other => panic!("not a build: {other:?}"),
    };
    assert_eq!((file.as_str(), emit), ("app.py", Some(EmitStage::Tyir)));
    assert_eq!(
        output.as_deref().map(camino::Utf8Path::as_str),
        Some("out.txt")
    );
    assert_eq!(generation.choice().unwrap().mode, BuildMode::RELEASE_SMALL);
}

#[test]
fn run_passes_the_arguments_after_two_dashes() {
    let (arguments, generation) = match parse(&["run", "app.py", "--release", "--", "-x", "two"]) {
        Ok(Command::Run {
            arguments,
            generation,
            ..
        }) => (arguments, generation),
        other => panic!("not a run: {other:?}"),
    };
    assert_eq!(arguments, ["-x", "two"]);
    assert_eq!(generation.choice().unwrap().mode, BuildMode::RELEASE);
}

#[test]
fn wrong_usage_is_refused_with_status_2() {
    for line in [
        &["build"][..],
        &["build", "app.py", "--emit=exe"],
        &["build", "app.py", "--release", "--release-small"],
        &["check", "app.py", "--python-floor", "3.11"],
        // An unknown command parses as an external one, refused when it runs (CLI-32).
        &["print"],
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
        &["print", "settings", "app.py", "--python-floor", "3.15"],
    ] {
        let settings = match parse(line).unwrap() {
            Command::Check { settings, .. }
            | Command::Build { settings, .. }
            | Command::Run { settings, .. }
            | Command::Print { settings, .. } => settings,
            other => panic!("{other:?} takes no settings"),
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

#[test]
fn cli_32_a_command_typy_does_not_have_is_an_external_one_with_its_arguments() {
    match parse(&["fmt", "--check", "app.py"]).unwrap() {
        Command::External(arguments) => assert_eq!(arguments, ["fmt", "--check", "app.py"]),
        other => panic!("{other:?}"),
    }
}

#[test]
fn cli_25_print_takes_what_to_write_and_refuses_anything_else() {
    match parse(&[
        "print",
        "target-cpus",
        "--target",
        "aarch64-unknown-linux-gnu",
    ])
    .unwrap()
    {
        Command::Print { what, target, .. } => {
            assert_eq!(what, crate::arguments::PrintRequest::TargetCpus);
            assert_eq!(
                target.triple,
                Some(typy_codegen::TargetTriple::Aarch64LinuxGnu)
            );
        }
        other => panic!("{other:?}"),
    }
    assert!(parse(&["print", "everything"]).is_err());
}

/// Returns what the code generation flags of a build choose.
fn choice(flags: &[&str]) -> Result<crate::arguments::Choice, String> {
    let line: Vec<&str> = ["build", "app.py"]
        .into_iter()
        .chain(flags.iter().copied())
        .collect();
    match parse(&line).unwrap() {
        Command::Build { generation, .. } => generation.choice(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn cli_20_each_level_is_a_mode_and_the_release_flags_are_two_of_them() {
    use typy_codegen::Optimization;
    for (flags, level) in [
        (&[][..], Optimization::None),
        (&["-O0"], Optimization::None),
        (&["-O1"], Optimization::Basic),
        (&["-O", "2"], Optimization::Speed),
        (&["--release"], Optimization::Speed),
        (&["-O3"], Optimization::Aggressive),
        (&["-Os"], Optimization::Small),
        (&["-Oz"], Optimization::Size),
        (&["--release-small"], Optimization::Size),
        (&["-C", "opt-level=3"], Optimization::Aggressive),
        (&["-O2", "--release"][..], Optimization::Speed),
    ] {
        assert_eq!(
            choice(flags).map(|choice| choice.mode),
            Ok(BuildMode::of(level)),
            "{flags:?}"
        );
    }
    assert!(parse(&["build", "app.py", "-O4"]).is_err());
    assert_eq!(
        choice(&["-O2", "--release-small"]).unwrap_err(),
        "--release-small and -O2 give two optimisation levels; give one"
    );
}

#[test]
fn cli_30_a_flag_and_its_key_are_one_setting_and_two_values_are_refused() {
    assert_eq!(
        choice(&["-O3", "-C", "opt-level=1"]).unwrap_err(),
        "-O3 and -C opt-level=1 give two optimisation levels; give one"
    );
    assert_eq!(
        choice(&["--target-cpu", "skylake", "-C", "target-cpu=znver4"]).unwrap_err(),
        "--target-cpu skylake and -C target-cpu=znver4 give two processors; give one"
    );
    let same = choice(&["--target-cpu", "skylake", "-C", "target-cpu=skylake"]).unwrap();
    assert_eq!(same.processor.as_deref(), Some("skylake"));
    let features = choice(&["-C", "target-feature=+avx2", "-C", "target-feature=-sse4a"]).unwrap();
    assert_eq!(features.features, "+avx2,-sse4a");
    for wrong in [
        "opt-level",
        "opt-level=fast",
        "target-cpu=",
        "inline=yes",
        "llvm-args=-x",
    ] {
        assert!(parse(&["build", "app.py", "-C", wrong]).is_err(), "{wrong}");
    }
}

/// Returns the warning levels `line` gives, written as `WarningLevels` writes them.
fn levels(line: &[&str]) -> String {
    let (arguments, matches) =
        Arguments::parse_with_matches(std::iter::once("typy").chain(line.iter().copied())).unwrap();
    arguments.global(&matches).warning_levels.to_string()
}

#[test]
fn warning_levels_apply_in_order_with_options_before_the_command_first() {
    let cases: [(&[&str], &str); 7] = [
        (&["check", "a.py"], "warn"),
        (&["--deny-warnings", "build", "a.py"], "deny"),
        (
            &["build", "a.py", "-A", "W0101", "--deny-warnings"],
            "deny W0101=allow",
        ),
        (
            &["-D", "W0101", "check", "a.py", "-W", "W0101", "-A", "W0301"],
            "warn W0301=allow",
        ),
        (
            &[
                "run", "a.py", "-D", "warnings", "-A", "W0401", "-W", "W0401",
            ],
            "deny W0401=warn",
        ),
        (&["-A", "warnings", "explain", "E0101"], "allow"),
        (&["run", "a.py", "--", "-D", "warnings"], "warn"),
    ];
    for (line, expected) in cases {
        assert_eq!(levels(line), expected, "{line:?}");
    }
}

#[test]
fn a_warning_level_names_a_warning_or_every_warning() {
    for line in [
        &["check", "a.py", "-D", "E0101"][..],
        &["check", "a.py", "-W"],
        &["-A", "all", "check", "a.py"],
    ] {
        assert!(
            Arguments::parse_with_matches(std::iter::once("typy").chain(line.iter().copied()))
                .is_err(),
            "{line:?}"
        );
    }
}
