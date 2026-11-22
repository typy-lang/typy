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
