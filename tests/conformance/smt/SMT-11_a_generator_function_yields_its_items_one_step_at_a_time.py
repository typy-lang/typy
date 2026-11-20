# typy-test: SMT-11, FUN-07
# typy-expect: accepted
from collections.abc import Generator, Iterable, Iterator


def count(n: int) -> Iterator[int]:
    i = 0
    while i < n:
        yield i
        i += 1


def words(text: str) -> Iterable[str]:
    for word in text.split():
        yield word.upper()


def pairs() -> Generator[tuple[int, str], None, None]:
    yield (1, "a")
    yield (2, "b")


print(list(count(4)), sum(count(5)), list(words("a bc def")))
for number, letter in pairs():
    print(number, letter)
it = count(2)
print(next(it), next(it))
try:
    next(it)
except StopIteration:
    print("stop")
print(repr(count(1)).split(" at ")[0], type(count(1)).__name__)
