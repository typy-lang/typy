//! The commands and options of ADR-0104 and ADR-0105, run as a user runs `typy`: `typy print`,
//! external commands, code generation, colour, warning levels, `run -c` and `-m`, and several
//! stages of `--emit`.

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
    // A Mac builds for macOS only, whose baseline is the M1.
    if cfg!(target_os = "macos") {
        let mac = stdout(&project.typy(
            &["print", "target-cpus", "--target", "aarch64-apple-darwin"],
            None,
        ));
        assert!(
            mac.starts_with("apple-m1 (baseline, the default)\n"),
            "{mac}"
        );
        return;
    }
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

#[test]
fn cli_22_timings_name_each_stage_of_a_build_and_the_total() {
    let project = Project::new();
    let output = project.typy(&["build", "app.py", "--timings"], None);
    assert_eq!(status(&output), 0, "{}", stderr(&output));
    let text = stderr(&output);
    let stages: Vec<&str> = text
        .lines()
        .map(|line| {
            let mut words = line.split_whitespace();
            let stage = words.next().unwrap();
            let seconds: f64 = words.next().unwrap().parse().unwrap();
            assert!(seconds >= 0.0, "{line}");
            assert_eq!(words.next(), Some("s"), "{line}");
            stage
        })
        .collect();
    assert_eq!(
        stages,
        ["load", "check", "lower", "generate", "link", "total"]
    );
    let check = stderr(&project.typy(&["check", "app.py", "--timings"], None));
    assert!(check.starts_with("load "), "{check}");
    assert!(!check.contains("link"), "{check}");
}

#[test]
fn cli_23_verbose_writes_the_link_and_the_program_typy_runs() {
    let project = Project::new();
    let build = project.typy(&["build", "app.py", "-v"], None);
    assert_eq!(status(&build), 0, "{}", stderr(&build));
    let link = stderr(&build);
    assert!(link.contains(" -o "), "{link}");
    assert!(link.contains("libtypy_runtime.a"), "{link}");
    let quiet = project.typy(&["build", "app.py"], None);
    assert_eq!(stderr(&quiet), "");
    let run = project.typy(&["run", "-v", "app.py", "--", "an argument"], None);
    assert_eq!(stdout(&run), "hello\n");
    let shown = stderr(&run);
    assert!(
        shown
            .lines()
            .any(|line| line.ends_with("/app 'an argument'")),
        "{shown}"
    );
}

#[test]
fn cli_24_save_temps_keeps_the_object_file_beside_the_executable() {
    let project = Project::new();
    let plain = project.typy(&["build", "app.py"], None);
    assert_eq!(status(&plain), 0, "{}", stderr(&plain));
    assert!(!project.root.join("app.o").exists());
    let kept = project.typy(&["build", "app.py", "--save-temps", "-o", "tool"], None);
    assert_eq!(status(&kept), 0, "{}", stderr(&kept));
    let object = std::fs::read(project.root.join("tool.o")).unwrap();
    let magic: &[u8] = if cfg!(target_os = "macos") {
        &[0xcf, 0xfa, 0xed, 0xfe]
    } else {
        b"\x7fELF"
    };
    assert!(object.starts_with(magic));
}

/// A program whose float arithmetic shows the processor's instruction encoding.
const FLOATS: &str = "def scale(values: list[float], factor: float) -> float:\n    total = 0.0\n    \
                      for value in values:\n        total += value * factor + 1.0\n    return total\n\n\n\
                      print(scale([1.5, 2.5, 3.5], 2.0))\n";

#[test]
fn cli_20_every_level_builds_a_program_that_runs_and_marks_its_size() {
    let project = Project::new();
    for level in ["-O0", "-O1", "-O2", "-O3", "-Os", "-Oz"] {
        let output = project.typy(&["run", level, "app.py"], None);
        assert_eq!(
            (status(&output), stdout(&output).as_str()),
            (0, "hello\n"),
            "{level}: {}",
            stderr(&output)
        );
    }
    let small = stdout(&project.typy(&["build", "app.py", "-Os", "--emit", "llvm-ir"], None));
    assert!(
        small.contains("optsize") && !small.contains("minsize"),
        "-Os"
    );
    let smallest = stdout(&project.typy(&["build", "app.py", "-Oz", "--emit", "llvm-ir"], None));
    assert!(smallest.contains("minsize"), "-Oz");
    let wrong = project.typy(&["build", "app.py", "-O3", "--release-small"], None);
    assert_eq!(
        (status(&wrong), stderr(&wrong)),
        (
            2,
            "error: --release-small and -O3 give two optimisation levels; give one\n".to_owned()
        )
    );
}

#[test]
fn cli_21_a_processor_is_checked_and_native_is_only_for_the_host() {
    let project = Project::new();
    std::fs::write(project.root.join("app.py"), FLOATS).unwrap();
    let native = project.typy(&["run", "-O3", "--target-cpu", "native", "app.py"], None);
    assert_eq!(stdout(&native), "18.0\n", "{}", stderr(&native));
    let unknown = project.typy(&["build", "app.py", "--target-cpu", "nosuch"], None);
    assert_eq!(status(&unknown), 2);
    assert!(
        stderr(&unknown).starts_with("error: `nosuch` is not a processor of "),
        "{}",
        stderr(&unknown)
    );
    // A Mac builds for itself only, so it has no other target to refuse `native` for.
    if cfg!(target_os = "macos") {
        return;
    }
    let host = stdout(&project.typy(&["print", "host"], None));
    let other = if host.starts_with("x86_64") {
        "aarch64-unknown-linux-gnu"
    } else {
        "x86_64-unknown-linux-gnu"
    };
    let elsewhere = project.typy(
        &[
            "build",
            "app.py",
            "--target-cpu",
            "native",
            "--target",
            other,
        ],
        None,
    );
    assert_eq!(status(&elsewhere), 2, "{}", stderr(&elsewhere));
    assert!(
        stderr(&elsewhere).contains("`native` is the processor of this machine"),
        "{}",
        stderr(&elsewhere)
    );
}

#[cfg(target_arch = "x86_64")]
#[test]
fn cli_21_a_processor_with_avx_encodes_float_arithmetic_with_vex() {
    let project = Project::new();
    std::fs::write(project.root.join("app.py"), FLOATS).unwrap();
    let assembly = |flags: &[&str]| {
        let mut line = vec!["build", "app.py", "-O2", "--emit", "asm"];
        line.extend_from_slice(flags);
        stdout(&project.typy(&line, None))
    };
    assert!(!assembly(&[]).contains("vaddsd"));
    assert!(assembly(&["--target-cpu", "haswell"]).contains("vaddsd"));
    assert!(assembly(&["-C", "target-feature=+avx"]).contains("vaddsd"));
}

#[test]
fn cli_30_codegen_help_lists_the_keys_and_needs_no_file() {
    let project = Project::new();
    let output = project.typy(&["-C", "help"], None);
    assert_eq!(status(&output), 0);
    let text = stdout(&output);
    for key in ["opt-level=LEVEL", "target-cpu=NAME", "target-feature=LIST"] {
        assert!(text.contains(&format!("-C {key}")), "{text}");
    }
    let built = project.typy(&["run", "-C", "opt-level=3", "app.py"], None);
    assert_eq!(stdout(&built), "hello\n", "{}", stderr(&built));
    let unknown = project.typy(&["build", "app.py", "-C", "llvm-args=-x"], None);
    assert_eq!(status(&unknown), 2);
}

#[test]
fn cli_33_each_level_and_processor_is_a_build_of_its_own_in_the_cache() {
    let project = Project::new();
    let cache = project.root.join("home").join("cache");
    let builds = || std::fs::read_dir(&cache).map_or(0, Iterator::count);
    for (flags, expected) in [
        (&[][..], 1),
        (&[][..], 1),
        (&["-O3"][..], 2),
        (&["-O3", "--target-cpu", "native"][..], 3),
        (&["-O3", "-C", "target-feature=-avx"][..], 4),
        (&["-O3"][..], 4),
    ] {
        let mut line = vec!["run"];
        line.extend_from_slice(flags);
        line.push("app.py");
        let output = project.typy(&line, None);
        assert_eq!(stdout(&output), "hello\n", "{flags:?}: {}", stderr(&output));
        assert_eq!(builds(), expected, "{flags:?}");
    }
}

/// A program with two warnings: an invalid escape, W0101, and an assertion that is always
/// true, W0301.
const WARNED: &str = "print('\\d')\nassert (1, 'always')\n";

/// Returns the header of each diagnostic written as text, such as `error[W0301]`.
fn headers(output: &Output) -> Vec<String> {
    stderr(output)
        .lines()
        .filter(|line| line.starts_with("warning[") || line.starts_with("error["))
        .map(|line| line.split(':').next().unwrap_or(line).to_owned())
        .collect()
}

#[test]
fn cli_26_color_always_colours_text_and_auto_leaves_a_pipe_plain() {
    let project = Project::new();
    std::fs::write(project.root.join("app.py"), WARNED).unwrap();
    let coloured = project.typy(&["--color", "always", "check", "app.py"], None);
    let text = stderr(&coloured);
    assert_eq!(status(&coloured), 0, "{text}");
    assert!(
        text.starts_with("\x1b[1;33mwarning[W0101]\x1b[0m\x1b[1m: "),
        "{text}"
    );
    assert!(
        text.contains(" \x1b[1;34m-->\x1b[0m app.py:1:8\n"),
        "{text}"
    );
    for arguments in [
        &["check", "app.py"][..],
        &["check", "app.py", "--color=never"],
        &["check", "app.py", "--color=always", "--error-format=json"],
    ] {
        let plain = project.typy(arguments, None);
        assert_eq!(status(&plain), 0, "{arguments:?}");
        assert!(!stderr(&plain).contains('\x1b'), "{arguments:?}");
        assert!(stderr(&plain).contains("W0301"), "{arguments:?}");
    }
    let wrong = project.typy(&["check", "app.py", "--color=sometimes"], None);
    assert_eq!(status(&wrong), 2, "{}", stderr(&wrong));
}

#[test]
fn cli_27_the_last_level_given_for_a_warning_decides_it() {
    let project = Project::new();
    std::fs::write(project.root.join("app.py"), WARNED).unwrap();
    let cases: [(&[&str], i32, &[&str]); 9] = [
        (
            &["check", "app.py"],
            0,
            &["warning[W0101]", "warning[W0301]"],
        ),
        (&["check", "app.py", "-A", "W0101"], 0, &["warning[W0301]"]),
        (&["-A", "warnings", "check", "app.py"], 0, &[]),
        (
            &["check", "app.py", "-D", "W0301"],
            1,
            &["warning[W0101]", "error[W0301]"],
        ),
        (
            &["-D", "warnings", "check", "app.py", "-A", "W0101"],
            1,
            &["error[W0301]"],
        ),
        (
            &["check", "app.py", "-A", "W0101", "-D", "warnings"],
            1,
            &["error[W0101]", "error[W0301]"],
        ),
        (
            &[
                "--deny-warnings",
                "check",
                "app.py",
                "-W",
                "W0101",
                "-A",
                "W0301",
            ],
            0,
            &["warning[W0101]"],
        ),
        (
            &["-D", "W0101", "check", "app.py", "-W", "W0101"],
            0,
            &["warning[W0101]", "warning[W0301]"],
        ),
        (
            &["check", "app.py", "-AW0101", "-W", "W0101", "-AW0301"],
            0,
            &["warning[W0101]"],
        ),
    ];
    for (arguments, expected_status, expected_headers) in cases {
        let output = project.typy(arguments, None);
        assert_eq!(
            (status(&output), headers(&output)),
            (
                expected_status,
                expected_headers
                    .iter()
                    .map(|header| (*header).to_owned())
                    .collect()
            ),
            "{arguments:?}: {}",
            stderr(&output)
        );
    }
}

#[test]
fn cli_27_a_denied_warning_fails_a_build_and_a_run_and_is_an_error_in_json() {
    let project = Project::new();
    std::fs::write(project.root.join("app.py"), WARNED).unwrap();
    let built = project.typy(&["build", "app.py", "-D", "W0301"], None);
    assert_eq!(status(&built), 1, "{}", stderr(&built));
    assert!(!project.root.join("app").exists());
    let quiet = project.typy(&["run", "app.py", "-A", "warnings"], None);
    assert_eq!(
        (
            status(&quiet),
            stdout(&quiet).as_str(),
            stderr(&quiet).as_str()
        ),
        (0, "\\d\n", "")
    );
    // The cached build of the run before must not stand in for one with a denied warning.
    let denied = project.typy(&["run", "app.py", "-D", "W0301"], None);
    assert_eq!((status(&denied), stdout(&denied).as_str()), (1, ""));
    let json = project.typy(
        &["check", "app.py", "-D", "W0301", "--error-format=json"],
        None,
    );
    let text = stderr(&json);
    assert_eq!(status(&json), 1, "{text}");
    assert!(
        text.contains("{\"code\":\"W0301\",\"severity\":\"error\","),
        "{text}"
    );
    assert!(
        text.contains("{\"code\":\"W0101\",\"severity\":\"warning\","),
        "{text}"
    );
}

#[test]
fn cli_27_a_level_for_an_error_or_an_unknown_code_is_wrong_usage() {
    let project = Project::new();
    for (option, code) in [("-D", "E0402"), ("-W", "W0999"), ("-A", "unused")] {
        let output = project.typy(&["check", "app.py", option, code], None);
        let text = stderr(&output);
        assert_eq!(status(&output), 2, "{text}");
        assert!(
            text.contains(&format!(
                "`{code}` names no warning; give `warnings` or one of W0101, W0301, W0401"
            )),
            "{text}"
        );
    }
}

/// Writes `files` into the project, each under its path from the project's directory.
fn write_files(project: &Project, files: &[(&str, &str)]) {
    for (path, text) in files {
        let path = project.root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

#[test]
fn cli_28_run_c_runs_code_that_imports_from_the_current_directory() {
    let project = Project::new();
    write_files(&project, &[("tools.py", "x = 41\n")]);
    let sum = project.typy(&["run", "-c", "import tools\nprint(tools.x + 1)"], None);
    assert_eq!(
        (status(&sum), stdout(&sum).as_str()),
        (0, "42\n"),
        "{}",
        stderr(&sum)
    );
    let argv = project.typy(
        &["run", "-c", "import sys\nprint(sys.argv)", "--", "a", "b"],
        None,
    );
    assert_eq!(stdout(&argv), "['-c', 'a', 'b']\n", "{}", stderr(&argv));
    // The code is part of the cache key, so other code is another build.
    let other = project.typy(&["run", "-c", "print(tools_x := 7)"], None);
    assert_eq!(stdout(&other), "7\n", "{}", stderr(&other));
}

#[test]
fn cli_28_diagnostics_of_code_name_it_string() {
    let project = Project::new();
    let output = project.typy(&["run", "-c", "print(undefined)"], None);
    let text = stderr(&output);
    assert_eq!(status(&output), 1, "{text}");
    assert!(text.contains(" --> <string>:1:7\n"), "{text}");
}

#[test]
fn cli_28_run_m_runs_a_module_after_its_packages_with_its_relative_imports() {
    let project = Project::new();
    write_files(
        &project,
        &[
            ("tools/__init__.py", "print('tools starts')\n"),
            (
                "tools/report.py",
                "import sys\nfrom . import numbers\n\
                 print(__name__, numbers.value, sys.argv[0].endswith('tools/report.py'), \
                 sys.argv[1:])\n",
            ),
            ("tools/numbers.py", "value = 1\n"),
            ("tools/__main__.py", "print('tools main')\n"),
        ],
    );
    let module = project.typy(&["run", "-m", "tools.report", "--", "one"], None);
    assert_eq!(
        (status(&module), stdout(&module).as_str()),
        (0, "tools starts\n__main__ 1 True ['one']\n"),
        "{}",
        stderr(&module)
    );
    let package = project.typy(&["run", "-m", "tools"], None);
    assert_eq!(
        (status(&package), stdout(&package).as_str()),
        (0, "tools starts\ntools main\n"),
        "{}",
        stderr(&package)
    );
}

#[test]
fn cli_28_a_module_that_cannot_run_is_an_error_and_two_mains_are_wrong_usage() {
    let project = Project::new();
    write_files(&project, &[("tools/__init__.py", "")]);
    let cases = [
        (
            &["run", "-m", "nothing"][..],
            1,
            "no module named `nothing`",
        ),
        (
            &["run", "-m", "tools"],
            1,
            "no module named `tools.__main__`; `tools` is a package and cannot be run directly",
        ),
        (&["run", "-m", "a..b"], 2, "`a..b` is not a module name"),
        (&["run"], 2, "<FILE|-c <CODE>|-m <MODULE>>"),
        (
            &["run", "app.py", "-c", "print(1)"],
            2,
            "cannot be used with",
        ),
        (
            &["run", "-c", "print(1)", "-m", "tools"],
            2,
            "cannot be used with",
        ),
    ];
    for (arguments, expected_status, expected_text) in cases {
        let output = project.typy(arguments, None);
        let text = stderr(&output);
        assert_eq!(status(&output), expected_status, "{arguments:?}: {text}");
        assert!(text.contains(expected_text), "{arguments:?}: {text}");
    }
}

#[test]
fn cli_29_several_stages_are_written_beside_the_output_and_one_goes_to_standard_output() {
    let project = Project::new();
    let all = project.typy(
        &[
            "build",
            "app.py",
            "-O2",
            "--emit=ast,typed-ast,tyir,tyir-mem,llvm-ir-unoptimised,llvm-ir",
            "--emit=llvm-bc,asm,obj",
        ],
        None,
    );
    assert_eq!(
        (status(&all), stdout(&all).as_str()),
        (0, ""),
        "{}",
        stderr(&all)
    );
    for extension in [
        "ast",
        "typed-ast",
        "tyir",
        "tyir-mem",
        "unoptimised.ll",
        "ll",
        "bc",
        "s",
        "o",
    ] {
        assert!(
            project.root.join(format!("app.{extension}")).is_file(),
            "app.{extension}"
        );
    }
    assert!(!project.root.join("app").exists());
    let read = |name: &str| std::fs::read(project.root.join(name)).unwrap();
    // LLVM wraps the bitcode of an Apple target in a header of its own, as clang does.
    let magic: &[u8] = if cfg!(target_os = "macos") {
        &[0xde, 0xc0, 0x17, 0x0b]
    } else {
        b"BC\xc0\xde"
    };
    assert!(read("app.bc").starts_with(magic));
    assert_ne!(read("app.unoptimised.ll"), read("app.ll"));
    let named = project.typy(
        &["build", "app.py", "--emit=llvm-ir,obj", "-o", "out/prog"],
        None,
    );
    std::fs::create_dir_all(project.root.join("out")).unwrap();
    assert_eq!(status(&named), 1, "the directory of -o does not exist yet");
    let named = project.typy(
        &["build", "app.py", "--emit=llvm-ir,obj", "-o", "out/prog"],
        None,
    );
    assert_eq!(status(&named), 0, "{}", stderr(&named));
    assert!(project.root.join("out/prog.ll").is_file());
    assert!(project.root.join("out/prog.o").is_file());
    let one = project.typy(&["build", "app.py", "--emit=llvm-ir-unoptimised"], None);
    assert_eq!(status(&one), 0, "{}", stderr(&one));
    assert!(
        stdout(&one).starts_with("; ModuleID = 'typy'"),
        "{}",
        stdout(&one)
    );
    let wrong = project.typy(&["build", "app.py", "--emit=llvm-ir,exe"], None);
    assert_eq!(status(&wrong), 2, "{}", stderr(&wrong));
}
