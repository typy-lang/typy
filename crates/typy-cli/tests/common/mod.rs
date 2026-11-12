//! What the end-to-end tests of built programs share: building a program with the `typy`
//! executable and running it as a subprocess.

use std::io::Write as _;
use std::process::{Command, Output, Stdio};

use camino::{Utf8Path, Utf8PathBuf};

/// A built program, in a directory of its own.
pub(crate) struct Program {
    directory: tempfile::TempDir,
    executable: Utf8PathBuf,
}

impl Program {
    /// Builds `source` as `app.py` in a new directory, which must succeed.
    pub(crate) fn build(source: &str) -> Self {
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
            directory,
            executable,
        }
    }

    /// Runs the program in its directory with `arguments` and `input` as its standard input.
    pub(crate) fn run(&self, arguments: &[&std::ffi::OsStr], input: &[u8]) -> Output {
        let mut child = Command::new(&self.executable)
            .args(arguments)
            .current_dir(self.root())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    }

    /// Returns the directory the program is built and run in.
    pub(crate) fn root(&self) -> &Utf8Path {
        Utf8Path::from_path(self.directory.path()).expect("the directory is UTF-8")
    }
}
