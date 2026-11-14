# typy-test: EXC-11
# typy-expect: accepted
def check(error: ValueError) -> None:
    print(error.args, str(error.args), repr(error.args), len(error.args), bool(error.args))
    print(f"{error.args} {error.args!r} {error.args = }")
    print(error.args == ("bad", 3), error.args != ("bad",), not error.args)
    if error.args:
        print("has arguments")


try:
    raise ValueError("bad", 3)
except ValueError as error:
    check(error)
try:
    raise ValueError("one")
except ValueError as error:
    print(error.args, error.args == ("one",), len(error.args))
try:
    raise ValueError()
except ValueError as error:
    print(error.args, len(error.args), bool(error.args), error.args == ())
try:
    int("x")
except ValueError as error:
    print(error.args, len(error.args))
maybe: int | str | None = None
try:
    raise KeyError(maybe)
except KeyError as error:
    print(error.args, error.args == (None,))
