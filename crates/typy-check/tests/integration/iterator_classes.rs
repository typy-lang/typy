//! Classes whose instances are iterators or iterables, as `typing` reads the protocols
//! (TYP-20, ADR-0065).

use crate::common::{app, dump, messages};

const COUNTDOWN: &str = "from collections.abc import Generator, Iterable, Iterator\n\n\n\
                         class Countdown:\n    def __init__(self, start: int) -> None:\n        \
                         self.current = start\n\n    def __iter__(self) -> \"Countdown\":\n        \
                         return self\n\n    def __next__(self) -> int:\n        \
                         if self.current == 0:\n            raise StopIteration\n        \
                         self.current -= 1\n        return self.current\n\n\n";

#[test]
fn typ_20_an_iterator_class_is_an_iterator_and_an_iterable_of_its_items() {
    let programs = [
        "steps: Iterator[int] = Countdown(2)\nevery: Iterable[int] = Countdown(1)\n\
         print(next(steps), list(every))\n",
        "class Word:\n    def __iter__(self) -> Iterator[str]:\n        return iter(\"ab\")\n\n\n\
         letters: Iterable[str] = Word()\nprint(sorted(letters))\n",
        "class Lines:\n    def __iter__(self) -> Generator[str, None, None]:\n        \
         yield \"x\"\n\n\nfor line in Lines():\n    print(line)\n\
         text: Iterable[str] = Lines()\n",
        "class Counts:\n    def __iter__(self) -> Countdown:\n        return Countdown(2)\n\n\n\
         numbers: Iterable[int] = Counts()\nprint(sum(numbers))\n",
        "class Faster(Countdown):\n    pass\n\n\nfast: Iterator[int] = Faster(3)\nfirst, *rest = fast\n",
    ];
    for program in programs {
        let source = format!("{COUNTDOWN}{program}");
        assert_eq!(app(&source), Vec::<String>::new(), "{source}");
    }
    let found = dump(&format!("{COUNTDOWN}steps: Iterator[int] = Countdown(2)\n"));
    assert!(found.contains("AsIterator"), "{found}");
}

#[test]
fn typ_20_a_class_that_does_not_fit_the_protocol_is_refused() {
    let refusals = [
        (
            "def maybe(values: Iterator[int | None]) -> None:\n    pass\n\n\nmaybe(Countdown(1))\n",
            "argument `values` must be an Iterator[int | None], and this is a Countdown",
        ),
        (
            "class Half:\n    def __next__(self) -> int:\n        return 1\n\n\n\
             steps: Iterator[int] = Half()\n",
            "`steps` was declared as Iterator[int] and is assigned a Half",
        ),
        (
            "class Wide:\n    def __iter__(self) -> \"Wide\":\n        return self\n\n    \
             def __next__(self, step: int = 1) -> int:\n        return step\n\n\n\
             steps: Iterator[int] = Wide()\n",
            "`steps` was declared as Iterator[int] and is assigned a Wide",
        ),
        (
            "class Listed:\n    def __iter__(self) -> list[int]:\n        return [1]\n\n\n\
             values: Iterable[int] = Listed()\n",
            "`values` was declared as Iterable[int] and is assigned a Listed",
        ),
    ];
    for (program, message) in refusals {
        let source = format!("{COUNTDOWN}{program}");
        let found = messages(&source);
        assert!(
            found.iter().any(|(_, text, _)| text == message),
            "{source}: {found:?}"
        );
    }
}
