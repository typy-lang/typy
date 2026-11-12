//! End-to-end tests of `sys` and `input` (LIB-04): a built program runs as a subprocess with
//! real arguments, piped standard input and its exit status, which the conformance suite's
//! oracle runs cannot vary.

use std::io::Write as _;
use std::process::{Command, Output, Stdio};

use camino::Utf8PathBuf;

/// A built program, in a directory of its own.
struct Program {
    _directory: tempfile::TempDir,
    executable: Utf8PathBuf,
}

impl Program {
    fn build(source: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
        std::fs::write(root.join("app.py"), source).unwrap();
        let executable = root.join("app");
        let built = Command::new(env!("CARGO_BIN_EXE_typy"))
            .args(["build", "app.py", "-o", "app"])
            .current_dir(&root)
            .env("TYPY_HOME", root.join("home"))
            .output()
            .unwrap();
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        Self {
            _directory: directory,
            executable,
        }
    }

    /// Runs the program with `arguments` and `input` as its standard input.
    fn run(&self, arguments: &[&std::ffi::OsStr], input: &[u8]) -> Output {
        let mut child = Command::new(&self.executable)
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn sys_argv_holds_the_executable_then_each_argument_as_given() {
    use std::os::unix::ffi::OsStrExt as _;
    let program = Program::build("import sys\nprint(sys.argv[1:], len(sys.argv))\n");
    let arguments = [
        std::ffi::OsStr::new("-x"),
        std::ffi::OsStr::new("two words"),
        std::ffi::OsStr::new(""),
        std::ffi::OsStr::new("caf\u{e9}"),
        std::ffi::OsStr::from_bytes(b"a\xffb"),
    ];
    let output = program.run(&arguments, b"");
    assert_eq!(
        text(&output.stdout),
        "['-x', 'two words', '', 'caf\u{e9}', 'a\\udcffb'] 6\n"
    );
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn input_reads_each_line_without_its_newline_until_the_end() {
    let program = Program::build(concat!(
        "count = 0\n",
        "while True:\n",
        "    try:\n",
        "        line = input(\"> \")\n",
        "    except EOFError:\n",
        "        break\n",
        "    count += 1\n",
        "    print(repr(line))\n",
        "print(count)\n",
    ));
    let output = program.run(&[], "one\n\ntwo\r\ncaf\u{e9}\nlast".as_bytes());
    assert_eq!(
        text(&output.stdout),
        "> 'one'\n> ''\n> 'two\\r'\n> 'caf\u{e9}'\n> 'last'\n> 5\n"
    );
}

#[test]
fn input_reads_a_line_longer_than_its_buffer() {
    let program = Program::build("line = input()\nprint(len(line), line[-3:])\n");
    let mut input = "x".repeat(10_000);
    input.push_str("end\nrest\n");
    let output = program.run(&[], input.as_bytes());
    assert_eq!(text(&output.stdout), "10003 end\n");
}

#[test]
fn the_prompt_of_input_is_written_before_reading() {
    let program = Program::build("print(\"before\", end=\"\")\nprint(input(42))\n");
    let output = program.run(&[], b"answer\n");
    assert_eq!(text(&output.stdout), "before42answer\n");
}

#[test]
fn sys_exit_gives_the_exit_status_and_writes_any_other_value() {
    for (source, status, error) in [
        ("import sys\nsys.exit()\n", 0, ""),
        ("import sys\nsys.exit(None)\n", 0, ""),
        ("import sys\nsys.exit(7)\n", 7, ""),
        ("import sys\nsys.exit(256 + 9)\n", 9, ""),
        ("import sys\nsys.exit(False)\n", 0, ""),
        ("import sys\nsys.exit(\"stopped\")\n", 1, "stopped\n"),
        ("raise SystemExit(1, 2)\n", 1, "(1, 2)\n"),
        (
            "class Done(SystemExit):\n    pass\n\n\nraise Done(4)\n",
            4,
            "",
        ),
    ] {
        let output = Program::build(source).run(&[], b"");
        assert_eq!(output.status.code(), Some(status), "{source}");
        assert_eq!(text(&output.stderr), error, "{source}");
    }
}

#[test]
fn standard_output_is_written_out_before_the_program_exits() {
    let program =
        Program::build("import sys\nprint(\"kept\")\nsys.stdout.write(\"x\")\nsys.exit(3)\n");
    let output = program.run(&[], b"");
    assert_eq!(text(&output.stdout), "kept\nx");
    assert_eq!(output.status.code(), Some(3));
}
