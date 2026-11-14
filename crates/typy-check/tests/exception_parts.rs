//! Tests of checking the parts of an exception a program reads through the public API
//! (EXC-11, EXC-13, EXC-15, EXC-16): `args`, `code`, and the fields of an `OSError`.

mod common;

use common::app;

#[test]
fn a_part_compares_from_either_side_of_the_operator() {
    let program = concat!(
        "try:\n",
        "    raise ValueError('bad')\n",
        "except ValueError as error:\n",
        "    print(('bad',) == error.args, () != error.args, error.args == ())\n",
    );
    assert_eq!(app(program), Vec::<String>::new());
}

#[test]
fn the_length_of_code_is_planned() {
    let program = concat!(
        "try:\n",
        "    raise SystemExit(3)\n",
        "except SystemExit as stop:\n",
        "    print(len(stop.code))\n",
    );
    assert_eq!(app(program), ["E0201"]);
}

#[test]
fn an_os_error_takes_a_file_name_that_is_a_str_or_none() {
    let cases = [
        ("str | None", Vec::<String>::new()),
        ("int | None", vec!["E0201".to_owned()]),
    ];
    for (annotation, expected) in cases {
        let program = format!(
            "def main(name: {annotation}) -> None:\n    print(OSError(2, 'x', name))\nmain(None)\n"
        );
        assert_eq!(app(&program), expected, "{annotation}");
    }
}

#[test]
fn a_subclass_reads_the_fields_of_os_error_unless_it_declares_them() {
    let program = concat!(
        "class Own(OSError):\n",
        "    def __init__(self) -> None:\n",
        "        super().__init__('own')\n",
        "        self.errno = 5\n",
        "class Plain(OSError):\n",
        "    pass\n",
        "def show(own: Own, plain: Plain) -> None:\n",
        "    print(own.errno + 1, plain.errno)\n",
        "show(Own(), Plain('plain'))\n",
    );
    assert_eq!(app(program), Vec::<String>::new());
}
