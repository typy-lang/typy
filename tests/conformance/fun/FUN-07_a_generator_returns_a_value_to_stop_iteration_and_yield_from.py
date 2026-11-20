# typy-test: FUN-07, SMT-11
# typy-expect: accepted
from collections.abc import Generator, Iterator


def inner() -> Generator[int, int | None, str]:
    got = yield 1
    print("inner got", got)
    got = yield 2
    print("inner got", got)
    return "inner result"


def outer() -> Generator[int, int | None, int]:
    result = yield from inner()
    print("outer result", result)
    yield from [10, 20]
    nothing = yield from iter([30])
    print("from a list", nothing)
    return 7


def guarded() -> Generator[int, None, None]:
    try:
        yield 1
        yield 2
    except ValueError as error:
        print("inner caught", error)
        yield 3
    finally:
        print("inner finally")


def delegating() -> Generator[int, None, None]:
    try:
        yield from guarded()
    finally:
        print("outer finally")


def listing() -> Generator[int, int, None]:
    yield from [1, 2]


def stops() -> Generator[int, None, int]:
    yield 1
    return 5


def returns_none() -> Generator[int, None, int | None]:
    yield 1
    return None


gen = outer()
print(next(gen))
print(gen.send(5))
print(gen.send(6))
print(next(gen))
print(next(gen))
try:
    next(gen)
except StopIteration as error:
    print("stop", error, repr(error), error.args)
d = delegating()
print(next(d))
print(d.throw(ValueError("bad")))
d.close()
s = stops()
next(s)
try:
    next(s)
except StopIteration as error:
    print("stops", repr(error), error.args)
r = returns_none()
next(r)
try:
    next(r)
except StopIteration as error:
    print("returns none", repr(error), error.args)
print(list(stops()))
for value in stops():
    print("loop", value)
t = listing()
next(t)
try:
    t.throw(KeyError("k"))
except KeyError as error:
    print("thrown through list", repr(error))
s2 = stops()
next(s2)
next(s2, 0)
print("default given")
x = stops()
next(x)
next(x)
