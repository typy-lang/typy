//! End-to-end tests of running out of memory: a built program prints `MemoryError` and
//! aborts, which a conformance test cannot compare, since CPython raises it instead.

use crate::common::Program;

#[test]
fn a_list_repeated_past_the_address_space_runs_out_of_memory() {
    let program = Program::build("print(len([1] * 8 * 2**61))\n");
    let output = program.run(&[], b"");
    assert_eq!(output.status.code(), None);
    assert_eq!(String::from_utf8_lossy(&output.stderr), "MemoryError\n");
}

#[test]
fn an_empty_list_repeated_any_number_of_times_stays_empty() {
    let program = Program::build("empty: list[int] = []\nprint(empty * 2**62, [1] * -2**62)\n");
    let output = program.run(&[], b"");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "[] []\n");
}
