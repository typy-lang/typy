//! End-to-end tests of the allocator a project names in `typy.toml` (MEM-08, ADR-0043): the
//! allocator is a C library the test builds, and the program is built and run as a developer
//! builds and runs it.

use std::process::{Command, Output};

use camino::Utf8PathBuf;

/// An allocator that counts its calls over the C library's and reports, when the program
/// ends, which kinds of call it saw; `refusing_allocate` has no memory to give.
const COUNTING: &str = r#"
#include <stdio.h>
#include <stdlib.h>

static long allocations, resizes, frees;

void *counting_allocate(size_t size) { allocations++; return malloc(size); }
void *counting_reallocate(void *pointer, size_t size) { resizes++; return realloc(pointer, size); }
void counting_free(void *pointer) { frees++; free(pointer); }
void *refusing_allocate(size_t size) { (void)size; return NULL; }

__attribute__((destructor)) static void report(void) {
    fprintf(stderr, "allocated %d, resized %d, freed %d\n",
            allocations > 0, resizes > 0, frees > 0);
}
"#;

/// A program that allocates, grows a list and frees what it made.
const PROGRAM: &str = "\
words: list[str] = []
for index in range(100):
    words.append(\"word \" + str(index))
print(len(words), words[-1])
";

/// A project directory with `app.py`, `typy.toml` and the allocator library in `lib/`.
struct Project {
    _directory: tempfile::TempDir,
    root: Utf8PathBuf,
}

impl Project {
    fn new(settings: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
        std::fs::write(root.join("app.py"), PROGRAM).unwrap();
        std::fs::write(root.join("typy.toml"), settings).unwrap();
        let library = root.join("lib");
        std::fs::create_dir(&library).unwrap();
        std::fs::write(library.join("counting.c"), COUNTING).unwrap();
        let compiled = Command::new("cc")
            .args(["-c", "-O2", "counting.c", "-o", "counting.o"])
            .current_dir(&library)
            .status()
            .unwrap();
        assert!(compiled.success());
        let archived = Command::new("ar")
            .args(["rcs", "libcounting.a", "counting.o"])
            .current_dir(&library)
            .status()
            .unwrap();
        assert!(archived.success());
        Self {
            _directory: directory,
            root,
        }
    }

    /// Builds `app.py` with `arguments` after `build app.py -o app`, the linker finding the
    /// library through `LIBRARY_PATH`.
    fn build(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_typy"))
            .args(["build", "app.py", "-o", "app"])
            .args(arguments)
            .current_dir(&self.root)
            .env("TYPY_HOME", self.root.join("home"))
            .env("LIBRARY_PATH", self.root.join("lib"))
            .output()
            .unwrap()
    }

    fn run(&self) -> Output {
        Command::new(self.root.join("app"))
            .current_dir(&self.root)
            .output()
            .unwrap()
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

const COUNTING_SETTINGS: &str = "\
[allocator]
allocate = \"counting_allocate\"
reallocate = \"counting_reallocate\"
free = \"counting_free\"
library = \"counting\"
";

#[test]
fn mem_08_a_program_allocates_through_the_functions_its_project_names() {
    let project = Project::new(COUNTING_SETTINGS);
    for mode in [&[][..], &["--release"]] {
        let built = project.build(mode);
        assert!(built.status.success(), "{}", text(&built.stderr));
        let run = project.run();
        assert_eq!(run.status.code(), Some(0), "{mode:?}");
        assert_eq!(text(&run.stdout), "100 word 99\n");
        assert_eq!(text(&run.stderr), "allocated 1, resized 1, freed 1\n");
    }
}

#[test]
fn mem_08_functions_of_a_library_the_program_links_anyway_need_no_library() {
    let project = Project::new(
        "[allocator]\nallocate = \"malloc\"\nreallocate = \"realloc\"\nfree = \"free\"\n",
    );
    let built = project.build(&[]);
    assert!(built.status.success(), "{}", text(&built.stderr));
    let run = project.run();
    assert_eq!(
        (run.status.code(), text(&run.stdout)),
        (Some(0), "100 word 99\n".to_owned())
    );
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn mem_08_an_allocator_without_memory_ends_the_program_with_memory_error() {
    let project =
        Project::new(&COUNTING_SETTINGS.replace("counting_allocate", "refusing_allocate"));
    let built = project.build(&[]);
    assert!(built.status.success(), "{}", text(&built.stderr));
    let run = project.run();
    assert_eq!(run.status.code(), None, "the program aborts");
    assert_eq!(text(&run.stdout), "");
    assert_eq!(text(&run.stderr), "MemoryError\n");
}

#[test]
fn mem_08_a_library_the_linker_cannot_find_fails_the_build_with_e0702() {
    let project = Project::new(
        &COUNTING_SETTINGS.replace("library = \"counting\"", "library = \"no_such_allocator\""),
    );
    let built = project.build(&[]);
    assert_eq!(built.status.code(), Some(1));
    let error = text(&built.stderr);
    assert!(error.starts_with("error[E0702]"), "{error}");
    assert!(error.contains("no_such_allocator"), "{error}");
    assert!(!project.root.join("app").exists());
}

#[test]
fn mem_08_a_wrong_allocator_table_stops_the_build_at_its_line() {
    let project =
        Project::new("[allocator]\nallocate = \"a\"\nreallocate = \"r\"\nfree = \"typy_free\"\n");
    let built = project.build(&[]);
    assert_eq!(built.status.code(), Some(2));
    let error = text(&built.stderr);
    assert!(error.starts_with("error[E0107]: "), "{error}");
    assert!(error.contains("typy.toml:4:8"), "{error}");
    assert!(error.contains("= rule: MEM-08"), "{error}");
}
