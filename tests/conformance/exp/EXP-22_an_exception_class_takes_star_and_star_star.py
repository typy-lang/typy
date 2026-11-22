# typy-test: EXP-22
# typy-expect: accepted
class Failure(Exception):
    pass


def make(values: list[int], options: dict[str, int]) -> str:
    try:
        error = ValueError(*values, **options)
        return f"{error} {error!r}"
    except TypeError as problem:
        return str(problem)


def fail(values: list[str], options: dict[str, int]) -> str:
    try:
        raise Failure("first", *values, **options)
    except Failure as error:
        return f"{error!r} {len(error.args)} {error.args}"
    except TypeError as problem:
        return str(problem)


def items(values: list[int]) -> None:
    try:
        raise ValueError(*values)
    except ValueError as error:
        for item in error.args:
            print("item", item)
        if error.args:
            print("first", error.args[0])


def stop(values: list[int]) -> None:
    try:
        raise SystemExit(*values)
    except SystemExit as error:
        code = error.code
        if isinstance(code, int):
            print("code", code + 1)
        elif code is None:
            print("no code")
        else:
            print("codes", len(code), code)


print(make([1, 2], {}), make([], {}), make([7], {}))
print(make([1], {"code": 2}))
print(fail(["a", "b"], {}), fail([], {}))
print(fail([], {"code": 1}))
items([3, 4])
items([])
for values in [[], [5], [6, 7]]:
    stop(values)
first = {"a": 1}
second = {"a": 2}
try:
    Failure(*[1], **first, **second)
except TypeError as problem:
    print(problem)
try:
    raise KeyError(*["key"])
except KeyError as error:
    print(error, repr(error), error.args)
