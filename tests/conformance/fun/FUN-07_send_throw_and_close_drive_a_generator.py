# typy-test: FUN-07, SMT-11, FUN-11
# typy-expect: accepted
from collections.abc import Generator


def accumulate() -> Generator[int, int, None]:
    total = 0
    while True:
        received = yield total
        if received is None:
            print("resumed by next")
        else:
            total += received


def echo() -> Generator[str, str | None, None]:
    reply: str | None = yield "ready"
    while reply is not None:
        reply = yield "got " + reply
    print("echo saw None")


def guarded() -> Generator[int, None, None]:
    try:
        yield 1
        yield 2
    except ValueError as error:
        print("caught", error)
        yield 99
    finally:
        print("finally runs")


def stubborn() -> Generator[int, None, None]:
    try:
        yield 1
    except GeneratorExit:
        yield 2


def pairs() -> Generator[int, tuple[int, str], None]:
    got = yield 0
    while got is not None:
        number, name = got
        print("pair", number, name)
        got = yield number


def main() -> None:
    counter = accumulate()
    print(next(counter))
    print(counter.send(5))
    print(counter.send(10))
    print(next(counter))
    counter.close()
    try:
        counter.send(1)
    except StopIteration:
        print("closed generator stops")
    talk = echo()
    print(next(talk))
    print(talk.send("hello"))
    try:
        talk.send(None)
    except StopIteration:
        print("echo ended")
    fresh = accumulate()
    try:
        fresh.send(3)
    except TypeError as error:
        print("TypeError:", error)
    print(next(fresh))
    safe = guarded()
    print(next(safe))
    print(safe.throw(ValueError("bad")))
    safe.close()
    other = guarded()
    print(next(other))
    try:
        other.throw(KeyError("k"))
    except KeyError as error:
        print("KeyError passed through", error)
    unstarted = guarded()
    try:
        unstarted.throw(ValueError)
    except ValueError:
        print("unstarted raises at once")
    unstarted.close()
    bad = stubborn()
    print(next(bad))
    try:
        bad.close()
    except RuntimeError as error:
        print("RuntimeError:", error)
    pair = pairs()
    print(next(pair))
    print(pair.send((7, "seven")))
    pair.close()


main()
