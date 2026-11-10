# typy-test: CST-04, EXC-01, STD-05
# typy-expect: accepted
def inner() -> None:
    try:
        raise KeyError("missing")
    except IndexError:
        print("not this one")


def outer() -> None:
    try:
        inner()
    except ValueError:
        print("nor this one")


try:
    outer()
except LookupError as error:
    print("caught at the top:", repr(error))
outer()
