//! End-to-end tests of text files (LIB-04, ADR-0019): what a built program leaves on disk
//! once it has ended, which a conformance test cannot look at.

mod common;

use common::Program;

fn contents(program: &Program, name: &str) -> String {
    std::fs::read_to_string(program.root().join(name)).unwrap()
}

#[test]
fn files_still_open_at_the_end_are_written_out() {
    let program = Program::build(concat!(
        "kept = open(\"kept.txt\", \"w\")\n",
        "kept.write(\"written at the end\")\n",
        "print(repr(open(\"kept.txt\").read()))\n",
    ));
    let output = program.run(&[], b"");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "''\n");
    assert_eq!(contents(&program, "kept.txt"), "written at the end");
}

#[test]
fn files_are_written_out_after_an_uncaught_exception_or_sys_exit() {
    for (ending, status) in [
        ("raise KeyError(\"stop\")\n", 1),
        ("import sys\nsys.exit(4)\n", 4),
    ] {
        let source = format!(
            "first = open(\"first.txt\", \"w\")\nfirst.write(\"one\")\nsecond = open(\"second.txt\", \"a\")\nsecond.write(\"two\")\n{ending}"
        );
        let program = Program::build(&source);
        let output = program.run(&[], b"");
        assert_eq!(output.status.code(), Some(status), "{ending}");
        assert_eq!(contents(&program, "first.txt"), "one", "{ending}");
        assert_eq!(contents(&program, "second.txt"), "two", "{ending}");
    }
}

#[test]
fn a_file_no_longer_referred_to_is_closed_and_written_out() {
    let program = Program::build(concat!(
        "def save(text: str) -> None:\n",
        "    file = open(\"saved.txt\", \"w\")\n",
        "    file.write(text)\n",
        "\n",
        "\n",
        "save(\"saved\")\n",
        "print(open(\"saved.txt\").read())\n",
    ));
    let output = program.run(&[], b"");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "saved\n");
}

#[test]
fn a_large_write_reaches_the_file_in_order() {
    let program = Program::build(concat!(
        "with open(\"large.txt\", \"w\") as large:\n",
        "    for number in range(5000):\n",
        "        large.write(str(number) + \"\\n\")\n",
        "count = 0\n",
        "total = 0\n",
        "for line in open(\"large.txt\"):\n",
        "    count += 1\n",
        "    total += int(line)\n",
        "print(count, total)\n",
    ));
    let output = program.run(&[], b"");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "5000 12497500\n");
    let mut expected = String::new();
    for number in 0..5000 {
        expected.push_str(&number.to_string());
        expected.push('\n');
    }
    assert_eq!(contents(&program, "large.txt"), expected);
}
