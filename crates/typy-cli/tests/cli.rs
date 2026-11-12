//! End-to-end tests: the `typy` executable runs as a subprocess, as a developer runs it.

use std::process::{Command, Output};

use camino::{Utf8Path, Utf8PathBuf};

/// A directory with the program `app.py`, and a private `TYPY_HOME`.
struct Workspace {
    _directory: tempfile::TempDir,
    root: Utf8PathBuf,
}

impl Workspace {
    fn new(source: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
        std::fs::write(root.join("app.py"), source).unwrap();
        Self {
            _directory: directory,
            root,
        }
    }

    fn typy(&self, arguments: &[&str]) -> Output {
        self.typy_at(Utf8Path::new(env!("CARGO_BIN_EXE_typy")), arguments)
    }

    fn typy_at(&self, compiler: &Utf8Path, arguments: &[&str]) -> Output {
        Command::new(compiler)
            .args(arguments)
            .current_dir(&self.root)
            .env("TYPY_HOME", self.root.join("home"))
            .output()
            .unwrap()
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

const HELLO: &str = "print(\"hello\", 1 + 2)\n";

#[test]
fn cli_06_wrong_usage_exits_with_status_2() {
    let workspace = Workspace::new(HELLO);
    for arguments in [
        &[][..],
        &["build"],
        &["build", "app.py", "--emit=exe"],
        &["launch"],
    ] {
        assert_eq!(status(&workspace.typy(arguments)), 2, "{arguments:?}");
    }
    let output = workspace.typy(&["build", "app.txt"]);
    assert_eq!(
        (status(&output), stderr(&output).as_str()),
        (2, "error: app.txt is not a `.py` file\n")
    );
    let version = workspace.typy(&["--version"]);
    assert_eq!(
        (status(&version), stdout(&version)),
        (0, format!("typy {}\n", env!("CARGO_PKG_VERSION")))
    );
}

#[test]
fn cli_03_check_reports_each_diagnostic_with_its_place() {
    let workspace = Workspace::new(HELLO);
    let clean = workspace.typy(&["check", "app.py"]);
    assert_eq!((status(&clean), stderr(&clean).as_str()), (0, ""));
    std::fs::write(workspace.root.join("app.py"), "x: int = 1\nx = \"a\"\n").unwrap();
    let wrong = workspace.typy(&["check", "app.py"]);
    assert_eq!(status(&wrong), 1);
    let text = stderr(&wrong);
    assert!(text.starts_with("error[E0402]: "), "{text}");
    assert!(text.contains(" --> app.py:2:"), "{text}");
    let missing = workspace.typy(&["check", "absent.py"]);
    assert_eq!(status(&missing), 1);
    assert!(
        stderr(&missing).starts_with("error[E0105]"),
        "{}",
        stderr(&missing)
    );
}

#[test]
fn cli_01_and_cli_05_each_mode_writes_an_executable_beside_the_main_file_or_at_the_output() {
    let workspace = Workspace::new(HELLO);
    for (arguments, path) in [
        (&["build", "app.py"][..], "app"),
        (
            &["build", "app.py", "-o", "small", "--release-small"],
            "small",
        ),
        (
            &["build", "app.py", "--release", "--output", "fast"],
            "fast",
        ),
    ] {
        let output = workspace.typy(arguments);
        assert_eq!(
            (status(&output), stderr(&output).as_str()),
            (0, ""),
            "{arguments:?}"
        );
        let run = Command::new(workspace.root.join(path)).output().unwrap();
        assert_eq!(stdout(&run), "hello 3\n");
    }
}

#[test]
fn nfr_06_the_same_build_gives_the_same_bytes() {
    let workspace = Workspace::new(HELLO);
    for output in ["first", "second"] {
        assert_eq!(
            status(&workspace.typy(&["build", "app.py", "-o", output])),
            0
        );
    }
    let first = std::fs::read(workspace.root.join("first")).unwrap();
    assert_eq!(first, std::fs::read(workspace.root.join("second")).unwrap());
}

#[test]
fn cli_04_emit_writes_one_stage_to_standard_output_or_a_file() {
    let workspace = Workspace::new(HELLO);
    let tyir = workspace.typy(&["build", "app.py", "--emit=tyir"]);
    assert_eq!(status(&tyir), 0);
    assert!(
        stdout(&tyir).contains("fn __typy_main() -> unit raises"),
        "{}",
        stdout(&tyir)
    );
    let object = workspace.typy(&["build", "app.py", "--emit=obj", "-o", "app.o"]);
    assert_eq!((status(&object), stdout(&object).as_str()), (0, ""));
    assert!(
        std::fs::read(workspace.root.join("app.o"))
            .unwrap()
            .starts_with(b"\x7fELF")
    );
    assert!(
        !workspace.root.join("app").exists(),
        "no executable is written"
    );
    let unwritable = workspace.typy(&["build", "app.py", "--emit=ast", "-o", "absent/out.txt"]);
    assert_eq!(status(&unwritable), 1);
    assert!(stderr(&unwritable).starts_with("error[E0106]: cannot write absent/out.txt"));
}

#[test]
fn cli_02_run_builds_once_and_passes_on_the_program_status() {
    let workspace = Workspace::new(HELLO);
    let first = workspace.typy(&["run", "app.py", "--", "an", "argument"]);
    assert_eq!((status(&first), stdout(&first).as_str()), (0, "hello 3\n"));
    let builds = || {
        std::fs::read_dir(workspace.root.join("home/cache"))
            .unwrap()
            .count()
    };
    assert_eq!(builds(), 1);
    assert_eq!(stdout(&workspace.typy(&["run", "app.py"])), "hello 3\n");
    assert_eq!(builds(), 1, "the second run reuses the build");
    std::fs::write(workspace.root.join("app.py"), "print(1 // 0)\n").unwrap();
    let failing = workspace.typy(&["run", "app.py"]);
    assert_eq!(status(&failing), 1);
    assert!(
        stderr(&failing).ends_with("ZeroDivisionError: division by zero\n"),
        "{}",
        stderr(&failing)
    );
    assert_eq!(builds(), 2);
}

#[test]
fn cli_02_run_reports_a_program_that_does_not_build() {
    let workspace = Workspace::new("print(undefined)\n");
    let output = workspace.typy(&["run", "app.py"]);
    assert_eq!(status(&output), 1);
    assert!(
        stderr(&output).starts_with("error[E0205]"),
        "{}",
        stderr(&output)
    );
    let missing = workspace.typy(&["run", "absent.py"]);
    assert!(
        stderr(&missing).starts_with("error[E0105]"),
        "{}",
        stderr(&missing)
    );
    assert_eq!(status(&workspace.typy(&["run", "app.txt"])), 2);
}

#[test]
fn cli_01_a_compiler_without_its_runtime_archive_says_so() {
    let workspace = Workspace::new(HELLO);
    let compiler = workspace.root.join("bin/typy");
    std::fs::create_dir_all(compiler.parent().unwrap()).unwrap();
    std::fs::copy(env!("CARGO_BIN_EXE_typy"), &compiler).unwrap();
    let output = workspace.typy_at(&compiler, &["build", "app.py"]);
    assert_eq!(status(&output), 1);
    assert!(
        stderr(&output).starts_with("error[E0703]: cannot read the runtime archive"),
        "{}",
        stderr(&output)
    );
    let check = workspace.typy_at(&compiler, &["check", "app.py"]);
    assert_eq!(status(&check), 0, "checking needs no archive");
}

/// CLI-11: environment variables a compiler is often expected to honour change nothing.
#[test]
fn cli_11_environment_variables_other_than_typy_home_leave_the_output_unchanged() {
    let workspace = Workspace::new(HELLO);
    assert_eq!(
        status(&workspace.typy(&["build", "app.py", "-o", "plain"])),
        0
    );
    let output = Command::new(env!("CARGO_BIN_EXE_typy"))
        .args(["build", "app.py", "-o", "varied"])
        .current_dir(&workspace.root)
        .env("TYPY_HOME", workspace.root.join("home"))
        .envs([
            ("CC", "false"),
            ("CFLAGS", "-O3 -DNDEBUG"),
            ("LDFLAGS", "-s"),
            ("RUSTFLAGS", "-C opt-level=3"),
            ("LANG", "de_DE.UTF-8"),
            ("LC_ALL", "C"),
            ("TZ", "Asia/Kolkata"),
            ("SOURCE_DATE_EPOCH", "1"),
            ("PYTHONHASHSEED", "7"),
            ("PYTHONPATH", "/nowhere"),
        ])
        .output()
        .unwrap();
    assert_eq!((status(&output), stderr(&output).as_str()), (0, ""));
    let plain = std::fs::read(workspace.root.join("plain")).unwrap();
    assert_eq!(plain, std::fs::read(workspace.root.join("varied")).unwrap());
}

/// CLI-02: the arguments after `--` reach the program. A 0.1 program cannot read them, so the
/// cached build is replaced with a script that prints its arguments and `typy run` reuses it.
#[test]
fn cli_02_run_starts_the_program_with_the_arguments_after_two_dashes() {
    use std::os::unix::fs::PermissionsExt as _;
    let workspace = Workspace::new(HELLO);
    assert_eq!(status(&workspace.typy(&["run", "app.py"])), 0);
    let build = std::fs::read_dir(workspace.root.join("home/cache"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("app");
    std::fs::write(&build, "#!/bin/sh\nprintf '[%s]' \"$@\"\nexit 3\n").unwrap();
    std::fs::set_permissions(&build, std::fs::Permissions::from_mode(0o755)).unwrap();
    let output = workspace.typy(&["run", "app.py", "--", "-x", "two words", "--", ""]);
    assert_eq!(stdout(&output), "[-x][two words][--][]");
    assert_eq!(status(&output), 3);
}

/// A workspace whose `PATH` holds only `bin/`, with `cc` replaced by `script` when given.
fn with_cc(workspace: &Workspace, script: Option<&str>) -> Output {
    use std::os::unix::fs::PermissionsExt as _;
    let bin = workspace.root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    if let Some(script) = script {
        std::fs::write(bin.join("cc"), script).unwrap();
        std::fs::set_permissions(bin.join("cc"), std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    Command::new(env!("CARGO_BIN_EXE_typy"))
        .args(["build", "app.py"])
        .current_dir(&workspace.root)
        .env("TYPY_HOME", workspace.root.join("home"))
        .env("PATH", &bin)
        .output()
        .unwrap()
}

#[test]
fn a_missing_c_toolchain_is_e0701_and_writes_no_file() {
    let workspace = Workspace::new(HELLO);
    let output = with_cc(&workspace, None);
    assert_eq!(status(&output), 1);
    assert!(
        stderr(&output).starts_with("error[E0701]"),
        "{}",
        stderr(&output)
    );
    assert!(!workspace.root.join("app").exists());
}

#[test]
fn a_failing_link_is_e0702_and_writes_no_file() {
    let workspace = Workspace::new(HELLO);
    let output = with_cc(
        &workspace,
        Some("#!/bin/sh\necho 'ld: cannot link' >&2\nexit 1\n"),
    );
    assert_eq!(status(&output), 1);
    let text = stderr(&output);
    assert!(text.starts_with("error[E0702]"), "{text}");
    assert!(text.contains("ld: cannot link"), "{text}");
    assert!(!workspace.root.join("app").exists());
}

#[test]
fn an_empty_program_builds_prints_nothing_and_exits_0() {
    let workspace = Workspace::new("");
    let output = workspace.typy(&["run", "app.py"]);
    assert_eq!(
        (
            status(&output),
            stdout(&output).as_str(),
            stderr(&output).as_str()
        ),
        (0, "", "")
    );
}

#[test]
fn lex_09_a_file_of_16_mib_is_read_and_one_byte_more_is_e0101() {
    const LIMIT: usize = 16 * 1024 * 1024;
    let line = "pass\n";
    let mut source = line.repeat(LIMIT / line.len());
    source.push_str(&" ".repeat(LIMIT - source.len()));
    assert_eq!(source.len(), LIMIT);
    let workspace = Workspace::new(&source);
    let exact = workspace.typy(&["check", "app.py"]);
    assert!(!stderr(&exact).contains("E0101"), "{}", stderr(&exact));
    source.push(' ');
    std::fs::write(workspace.root.join("app.py"), &source).unwrap();
    let over = workspace.typy(&["check", "app.py"]);
    assert_eq!(status(&over), 1);
    assert!(
        stderr(&over).starts_with("error[E0101]"),
        "{}",
        stderr(&over)
    );
}

#[test]
fn lex_01_source_that_is_not_utf8_is_e0102() {
    let workspace = Workspace::new("");
    std::fs::write(workspace.root.join("app.py"), b"x = 1\ny = \"\xff\"\n").unwrap();
    let output = workspace.typy(&["check", "app.py"]);
    assert_eq!(status(&output), 1);
    let text = stderr(&output);
    assert!(text.starts_with("error[E0102]"), "{text}");
    assert!(text.contains("app.py:2:"), "{text}");
}

#[test]
fn nfr_06_two_builds_at_once_into_different_outputs_give_the_same_bytes() {
    let workspace = Workspace::new(HELLO);
    std::thread::scope(|scope| {
        for output in ["one", "two"] {
            let workspace = &workspace;
            scope.spawn(move || {
                assert_eq!(
                    status(&workspace.typy(&["build", "app.py", "-o", output])),
                    0
                );
            });
        }
    });
    let one = std::fs::read(workspace.root.join("one")).unwrap();
    assert_eq!(one, std::fs::read(workspace.root.join("two")).unwrap());
}

/// Runs `typy` in the workspace with only the environment variables given, `PATH`, and the
/// `LLVM_*` variables that coverage measurement sets.
fn typy_with_environment(
    workspace: &Workspace,
    arguments: &[&str],
    variables: &[(&str, &str)],
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_typy"))
        .args(arguments)
        .current_dir(&workspace.root)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap())
        // Coverage of the subprocess is written where these variables say (NFR-11).
        .envs(std::env::vars().filter(|(name, _)| name.starts_with("LLVM_")))
        .envs(variables.iter().copied())
        .output()
        .unwrap()
}

#[test]
fn cli_02_without_typy_home_builds_go_to_the_user_cache_directory() {
    let workspace = Workspace::new(HELLO);
    let xdg = workspace.root.join("xdg");
    let home = workspace.root.join("user");
    let run = typy_with_environment(
        &workspace,
        &["run", "app.py"],
        &[("XDG_CACHE_HOME", xdg.as_str()), ("HOME", home.as_str())],
    );
    assert_eq!(stdout(&run), "hello 3\n");
    assert!(xdg.join("typy").is_dir() && !home.exists());
    let run = typy_with_environment(&workspace, &["run", "app.py"], &[("HOME", home.as_str())]);
    assert_eq!(stdout(&run), "hello 3\n");
    assert!(home.join(".cache/typy").is_dir());
}

#[test]
fn cli_02_a_cache_that_cannot_be_created_is_e0106() {
    let workspace = Workspace::new(HELLO);
    let blocked = workspace.root.join("file");
    std::fs::write(&blocked, "").unwrap();
    let output = typy_with_environment(
        &workspace,
        &["run", "app.py"],
        &[("TYPY_HOME", blocked.as_str())],
    );
    assert_eq!(status(&output), 1);
    assert!(
        stderr(&output).starts_with("error[E0106]"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_c_toolchain_that_cannot_be_started_is_e0702() {
    let workspace = Workspace::new(HELLO);
    std::fs::create_dir_all(workspace.root.join("bin/cc")).unwrap();
    let output = with_cc(&workspace, None);
    assert_eq!(status(&output), 1);
    assert!(
        stderr(&output).starts_with("error[E0702]"),
        "{}",
        stderr(&output)
    );
    assert!(!workspace.root.join("app").exists());
}

#[test]
fn cli_12_python_floor_in_typy_toml_sets_the_floor_and_the_flag_overrides_it() {
    let workspace = Workspace::new("lazy import tools\nprint(tools.NAME)\n");
    std::fs::write(workspace.root.join("tools.py"), "NAME = 'tools'\n").unwrap();
    let default = workspace.typy(&["check", "app.py"]);
    assert_eq!(status(&default), 1);
    assert!(
        stderr(&default).starts_with("error[E0103]"),
        "{}",
        stderr(&default)
    );
    std::fs::write(
        workspace.root.join("typy.toml"),
        "python-floor = \"3.15\"\n",
    )
    .unwrap();
    let run = workspace.typy(&["run", "app.py"]);
    assert_eq!((status(&run), stdout(&run).as_str()), (0, "tools\n"));
    let overridden = workspace.typy(&["check", "app.py", "--python-floor", "3.14"]);
    assert_eq!(status(&overridden), 1);
    assert!(stderr(&overridden).starts_with("error[E0103]"));
}

#[test]
fn cli_07_a_typy_toml_that_is_not_valid_stops_with_status_2_at_its_line() {
    let workspace = Workspace::new(HELLO);
    for (settings, place, text) in [
        ("python-floor = 3.15\n", "typy.toml:1:16", "is a float"),
        (
            "python-floor = \"3.11\"\n",
            "typy.toml:1:16",
            "the string \"3.11\"",
        ),
        ("\n[x\n", "typy.toml:2:3", "not valid TOML"),
        ("floor = \"3.15\"\n", "typy.toml:1:1", "no setting `floor`"),
        (
            "module-paths = [\"../outside\"]\n",
            "typy.toml:1:17",
            "under the project",
        ),
        (
            "module-paths = [\"missing\"]\n",
            "typy.toml:1:17",
            "is not a directory",
        ),
        ("module-paths = \"lib\"\n", "typy.toml:1:16", "not an array"),
        ("module-paths = [1]\n", "typy.toml:1:17", "is an integer"),
    ] {
        std::fs::write(workspace.root.join("typy.toml"), settings).unwrap();
        let output = workspace.typy(&["build", "app.py"]);
        let error = stderr(&output);
        assert_eq!(status(&output), 2, "{settings}{error}");
        assert!(error.starts_with("error[E0107]: "), "{error}");
        assert!(error.contains(place) && error.contains(text), "{error}");
        assert!(!workspace.root.join("app").exists());
    }
}

#[test]
fn mod_01_modules_are_found_in_the_module_paths_after_the_main_files_directory() {
    let workspace = Workspace::new("import tools\nimport extra\nprint(tools.NAME, extra.NAME)\n");
    for (path, name) in [
        ("lib/tools.py", "lib"),
        ("vendor/tools.py", "vendor"),
        ("vendor/extra.py", "vendor"),
    ] {
        let path = workspace.root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!("NAME = '{name}'\n")).unwrap();
    }
    let settings = "module-paths = [\"lib\", \"vendor\"]\n";
    std::fs::write(workspace.root.join("typy.toml"), settings).unwrap();
    let run = workspace.typy(&["run", "app.py"]);
    assert_eq!((status(&run), stdout(&run).as_str()), (0, "lib vendor\n"));
    std::fs::write(workspace.root.join("tools.py"), "NAME = 'root'\n").unwrap();
    let run = workspace.typy(&["run", "app.py"]);
    assert_eq!(stdout(&run), "root vendor\n");
}

#[test]
fn dia_05_json_diagnostics_are_one_object_per_line() {
    let workspace = Workspace::new("x: int = 1\nx = \"a\"\nprint(y)\n");
    let output = workspace.typy(&["check", "app.py", "--error-format=json"]);
    assert_eq!(status(&output), 1);
    let text = stderr(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert!(
        lines[0].starts_with(
            "{\"code\":\"E0402\",\"severity\":\"error\",\"message\":\"`x` was declared as int"
        ),
        "{text}"
    );
    assert!(
        lines[0].contains("\"file\":\"app.py\",\"line\":2,\"column\":5,"),
        "{text}"
    );
    assert!(lines[1].starts_with("{\"code\":\"E0205\""), "{text}");
    assert!(lines.iter().all(|line| line.ends_with('}')), "{text}");
    std::fs::write(workspace.root.join("typy.toml"), "floor = 1\n").unwrap();
    let settings = workspace.typy(&["--error-format", "json", "build", "app.py"]);
    assert_eq!(status(&settings), 2);
    assert!(
        stderr(&settings).starts_with("{\"code\":\"E0107\""),
        "{}",
        stderr(&settings)
    );
    std::fs::remove_file(workspace.root.join("typy.toml")).unwrap();
    std::fs::write(workspace.root.join("app.py"), HELLO).unwrap();
    let clean = workspace.typy(&["run", "app.py", "--error-format=json"]);
    assert_eq!((status(&clean), stderr(&clean).as_str()), (0, ""));
}

#[test]
fn dia_07_warnings_are_shown_and_fail_only_with_deny_warnings() {
    let workspace = Workspace::new("print('\\d')\nassert (1, 'always')\n");
    let built = workspace.typy(&["build", "app.py"]);
    let text = stderr(&built);
    assert_eq!(status(&built), 0, "{text}");
    assert!(
        text.starts_with("warning[W0101]: `\\d` is an invalid escape sequence"),
        "{text}"
    );
    assert!(text.contains("warning[W0301]: "), "{text}");
    assert!(workspace.root.join("app").exists());
    let checked = workspace.typy(&["check", "app.py", "--error-format=json"]);
    assert_eq!(status(&checked), 0);
    assert_eq!(stderr(&checked).lines().count(), 2, "{}", stderr(&checked));
    for _ in 0..2 {
        let run = workspace.typy(&["run", "app.py"]);
        assert_eq!((status(&run), stdout(&run).as_str()), (0, "\\d\n"));
        assert!(
            stderr(&run).starts_with("warning[W0101]"),
            "{}",
            stderr(&run)
        );
    }
    std::fs::remove_file(workspace.root.join("app")).unwrap();
    for command in ["check", "build", "run"] {
        let denied = workspace.typy(&[command, "app.py", "--deny-warnings"]);
        assert_eq!(status(&denied), 1, "{command}");
        assert!(
            stderr(&denied).starts_with("warning[W0101]"),
            "{}",
            stderr(&denied)
        );
    }
    assert!(!workspace.root.join("app").exists());
}
