# typy-test: EXP-22
# typy-expect: accepted
class Coded(Exception):
    def __init__(self, code: int, *more: int) -> None:
        super().__init__(code)
        self.code = code
        self.more = more


class Plain(Exception):
    def __init__(self, first: int, second: int = 0) -> None:
        self.total = first + second


class Named(Exception):
    def __init__(self, text: str, *, level: int = 1) -> None:
        self.level = level


class Missing(OSError):
    def __init__(self, name: str) -> None:
        super().__init__(2, "missing", name)


def make(values: list[int], options: dict[str, int]) -> str:
    try:
        error = Plain(*values, **options)
        return f"{error.total} {error!r} {error.args}"
    except TypeError as problem:
        return str(problem)


print(make([1], {}), make([1, 2], {}), make([], {"first": 5}))
print(make([1, 2, 3], {}))
print(make([], {}))
print(make([1], {"first": 2}))
codes = [7, 8, 9]
coded = Coded(*codes)
print(coded.code, coded.more, coded.args, repr(coded))
levels = {"level": 3}
named = Named("hi", **levels)
print(named.level, named.args)
texts = ["data.txt"]
missing = Missing(*texts)
print(missing, missing.args, missing.filename)
second = {"second": 2}
third = {"second": 3}
try:
    Plain(*[1], **second, **third)
except TypeError as problem:
    print(problem)
