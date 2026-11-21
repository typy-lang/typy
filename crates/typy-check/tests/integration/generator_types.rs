//! Generator types: what a generator yields, is sent and returns, compared as values.

use crate::common::messages;

#[test]
fn a_generator_of_another_type_is_refused() {
    let refused = [
        (
            "from collections.abc import Generator\n\n\n\
            def numbers() -> Generator[int, None, None]:\n    yield 1\n\n\n\
            def words() -> Generator[str, None, None]:\n    yield \"a\"\n\n\n\
            source: Generator[int, None, None] = numbers()\nsource = words()\n",
            "E0402",
            "`source` was declared as Generator[int, None, None] and is assigned a \
            Generator[str, None, None]",
        ),
        (
            "from collections.abc import Generator\n\n\n\
            def numbers() -> Generator[int, None, None]:\n    yield 1\n\n\n\
            def echo() -> Generator[int, str, None]:\n    text = yield 1\n    print(text)\n\
            \n\nsource: Generator[int, None, None] = numbers()\nsource = echo()\n",
            "E0402",
            "`source` was declared as Generator[int, None, None] and is assigned a \
            Generator[int, str, None]",
        ),
        (
            "source = (number.nope() for number in [1, 2])\nsource.close()\n",
            "E0412",
            "an int has no method `nope` in TyPy",
        ),
        (
            "from collections.abc import Generator\nfrom typing import Protocol\n\n\n\
            def numbers() -> Generator[int, Protocol, None]:\n    yield 1\n\n\n\
            numbers().close()\ncount: int = \"a\"\n",
            "E0408",
            "`Protocol` is not a type; a protocol class derives from it",
        ),
    ];
    for (source, code, message) in refused {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{source}\n{found:?}"
        );
    }
}
