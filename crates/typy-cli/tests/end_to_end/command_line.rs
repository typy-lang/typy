//! The commands and options of ADR-0104 and ADR-0105, run as a user runs `typy`: `typy print`
//! and external commands.

use std::process::{Command, Output};

use camino::{Utf8Path, Utf8PathBuf};

/// A directory with `app.py`, and a private `TYPY_HOME`.
struct Project {
    _directory: tempfile::TempDir,
    root: Utf8PathBuf,
}

impl Project {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
        std::fs::write(root.join("app.py"), "print('hello')\n").unwrap();
        Self {
            _directory: directory,
            root,
        }
    }

    /// Runs `typy` in the project, with `PATH` set to `path` when it is given.
    fn typy(&self, arguments: &[&str], path: Option<&Utf8Path>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_typy"));
        command
            .args(arguments)
            .current_dir(&self.root)
            .env("TYPY_HOME", self.root.join("home"));
        if let Some(path) = path {
            command.env("PATH", path);
        }
        command.output().unwrap()
    }
}

fn status(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn cli_25_print_names_the_targets_the_host_and_the_versions() {
    let project = Project::new();
    let targets = project.typy(&["print", "targets"], None);
    assert_eq!(status(&targets), 0, "{}", stderr(&targets));
    let text = stdout(&targets);
    for triple in [
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        assert!(text.contains(triple), "{text}");
    }
    assert_eq!(text.matches(" (host)").count(), 1, "{text}");
    let host = stdout(&project.typy(&["print", "host"], None));
    assert!(text.contains(&format!("{} (host)", host.trim())), "{text}");
    let version = stdout(&project.typy(&["print", "version"], None));
    assert!(
        version.starts_with(&format!("typy = \"{}\"\n", env!("CARGO_PKG_VERSION"))),
        "{version}"
    );
    assert!(version.contains("llvm = \"23."), "{version}");
    assert!(
        version.contains("python-floors = [\"3.12\", \"3.13\", \"3.14\", \"3.15\"]"),
        "{version}"
    );
}

#[test]
fn cli_25_print_paths_names_the_runtime_archives_the_compiler_links() {
    let project = Project::new();
    let output = project.typy(&["print", "paths"], None);
    assert_eq!(status(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    let value = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("{key} = \"")))
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or_else(|| panic!("{key} missing from {text}"))
            .to_owned()
    };
    assert_eq!(value("compiler"), env!("CARGO_BIN_EXE_typy"));
    for (key, directory) in [
        ("runtime", "/lib/typy/"),
        ("runtime-small", "/lib/typy-small/"),
    ] {
        let archive = value(key);
        assert!(archive.contains(directory), "{archive}");
        assert!(Utf8Path::new(&archive).is_file(), "{archive} is not a file");
    }
    assert_eq!(
        value("cache"),
        project.root.join("home").join("cache").as_str()
    );
}

#[test]
fn cli_25_print_settings_writes_typy_toml_with_the_command_line_applied() {
    let project = Project::new();
    std::fs::write(
        project.root.join("typy.toml"),
        "python-floor = \"3.13\"\nmodule-paths = [\"lib\"]\n\n[libraries.z]\nlink = \"static\"\n",
    )
    .unwrap();
    std::fs::create_dir(project.root.join("lib")).unwrap();
    let output = project.typy(&["print", "settings", "app.py"], None);
    assert_eq!(status(&output), 0, "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "python-floor = \"3.13\"\nmodule-paths = [\"lib\"]\nlibrary-paths = []\n\
         libraries.z.link = \"static\"\n"
    );
    let floor = project.typy(
        &["print", "settings", "app.py", "--python-floor", "3.15"],
        None,
    );
    assert!(
        stdout(&floor).starts_with("python-floor = \"3.15\"\n"),
        "{}",
        stdout(&floor)
    );
    std::fs::write(project.root.join("typy.toml"), "python-floor = \"2.7\"\n").unwrap();
    // A file `typy check` refuses is refused alike, with its status.
    let wrong = project.typy(&["print", "settings", "app.py"], None);
    assert_eq!(
        status(&wrong),
        status(&project.typy(&["check", "app.py"], None))
    );
    assert!(stderr(&wrong).contains("E0107"), "{}", stderr(&wrong));
}

#[test]
fn cli_25_print_target_cpus_lists_the_baseline_native_and_llvms_processors() {
    let project = Project::new();
    let output = project.typy(
        &[
            "print",
            "target-cpus",
            "--target",
            "x86_64-unknown-linux-gnu",
        ],
        None,
    );
    assert_eq!(status(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("x86-64 (baseline, the default)"));
    let names: Vec<&str> = lines.collect();
    for name in ["skylake", "znver4", "x86-64-v3"] {
        assert!(names.contains(&name), "{name} missing from {text}");
    }
    let arm = stdout(&project.typy(
        &[
            "print",
            "target-cpus",
            "--target",
            "aarch64-unknown-linux-gnu",
        ],
        None,
    ));
    assert!(
        arm.starts_with("generic (baseline, the default)\n"),
        "{arm}"
    );
    assert!(arm.lines().any(|line| line == "neoverse-v2"), "{arm}");
}

#[cfg(unix)]
#[test]
fn cli_32_an_unknown_command_runs_typy_name_from_path_with_its_arguments_and_status() {
    use std::os::unix::fs::PermissionsExt as _;
    let project = Project::new();
    let tools = project.root.join("tools");
    std::fs::create_dir(&tools).unwrap();
    let program = tools.join("typy-hello");
    std::fs::write(
        &program,
        "#!/bin/sh\necho \"hello $*\"\necho \"compiler $TYPY\"\nexit 3\n",
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{tools}:/usr/bin:/bin");
    let output = project.typy(&["hello", "a", "--b"], Some(Utf8Path::new(&path)));
    assert_eq!(
        (status(&output), stdout(&output)),
        (
            3,
            format!("hello a --b\ncompiler {}\n", env!("CARGO_BIN_EXE_typy"))
        )
    );
}

#[test]
fn cli_32_an_unknown_command_without_a_program_is_wrong_usage() {
    let project = Project::new();
    let output = project.typy(&["frobnicate"], Some(project.root.as_path()));
    assert_eq!(status(&output), 2);
    assert_eq!(
        stderr(&output),
        "error: `frobnicate` is not a command of typy, and no `typy-frobnicate` was found \
         on PATH\n"
    );
    let path = project.typy(&["../escape"], None);
    assert_eq!(
        (status(&path), stderr(&path)),
        (
            2,
            "error: `../escape` is not a command of typy\n".to_owned()
        )
    );
}
