//! Tests of `typing.cast` checked when the program runs (TYP-24, ADR-0091): a wrong cast
//! raises `TypeError`, which CPython, trusting the cast, does not, so the conformance suite
//! cannot compare it.

use typy_build::BuildRequest;
use typy_driver::Outcome;

use crate::pipeline::{pipeline, program};

#[test]
fn typ_24_a_wrong_cast_raises_type_error_naming_the_type_and_the_class() {
    let source = "from typing import cast\n\
                  value: object = 'ada'\n\
                  try:\n    cast(int, value)\nexcept TypeError as error:\n    print(error)\n\
                  flag: object = True\n\
                  try:\n    cast(int, flag)\nexcept TypeError as error:\n    print(error)\n\
                  mixed: int | str = 'x'\n\
                  try:\n    cast(int, mixed)\nexcept TypeError as error:\n    print(error)\n\
                  print(cast(bool, flag), cast(str, mixed), cast(object, 3) == 3)\n";
    let (_guard, main) = program(source);
    let request = BuildRequest::new(&main).unwrap();
    let Ok(Outcome::Executable(artifact)) = pipeline().run(&request).map(|success| success.outcome)
    else {
        panic!("no executable; run `make runtime`");
    };
    let output = std::process::Command::new(&artifact.path).output().unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "cast to int found str\ncast to int found bool\ncast to int found str\nTrue x True\n"
    );
}
