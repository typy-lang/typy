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
fn the_length_of_code_is_its_type_s_length() {
    let cases = [
        ("raise SystemExit(3)", vec!["E0404".to_owned()]),
        ("raise SystemExit(3, 'stop')", Vec::<String>::new()),
    ];
    for (raise, expected) in cases {
        let program =
            format!("try:\n    {raise}\nexcept SystemExit as stop:\n    print(len(stop.code))\n");
        assert_eq!(app(&program), expected, "{raise}");
    }
}

#[test]
fn args_and_code_are_typed_by_every_creation_of_their_class() {
    let cases = [
        (
            "raise TypeError('bad', 3)",
            "x: tuple[str] | tuple[str, int] = error.args",
        ),
        ("raise SystemExit", "x: None = error.code"),
        ("raise SystemExit('stop')", "x: str = error.code"),
        (
            "raise KeyError(1.5)",
            "x: tuple[str] | tuple[float] = error.args",
        ),
    ];
    for (raise, read) in cases {
        let class = if raise.contains("SystemExit") {
            "SystemExit"
        } else {
            raise[6..raise.find('(').unwrap_or(raise.len())].trim()
        };
        let program =
            format!("try:\n    {raise}\nexcept {class} as error:\n    {read}\n    print(x)\n");
        assert_eq!(app(&program), Vec::<String>::new(), "{program}");
    }
}

#[test]
fn an_os_error_takes_a_file_name_of_any_type_but_a_number() {
    let cases = [
        ("str | None", Vec::<String>::new()),
        ("list[int] | None", Vec::<String>::new()),
        ("int | None", vec!["E0201".to_owned()]),
        ("float", vec!["E0201".to_owned()]),
    ];
    for (annotation, expected) in cases {
        let program =
            format!("def main(name: {annotation}) -> None:\n    print(OSError(2, 'x', name))\n");
        assert_eq!(app(&program), expected, "{annotation}");
    }
}

#[test]
fn a_subclass_of_os_error_takes_a_number_as_its_file_name() {
    let program = concat!(
        "class Busy(BlockingIOError):\n",
        "    pass\n",
        "print(Busy(11, 'x', 5).filename, FileNotFoundError(2, 'x', 3).filename)\n",
    );
    assert_eq!(app(program), Vec::<String>::new());
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
