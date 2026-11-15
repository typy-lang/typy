# typy-test: EXC-15
# typy-expect: accepted
class AppError(Exception):
    pass


def fail(kind: int) -> None:
    if kind == 0:
        raise AppError("bad", 3)
    if kind == 1:
        raise AppError("worse")
    raise AppError


for kind in range(3):
    try:
        fail(kind)
    except AppError as error:
        items = error.args
        print(items, len(items))
        for item in error.args:
            print("item", item)
        if len(error.args) > 0:
            print("first", error.args[0])

try:
    int("x")
except ValueError as error:
    message = error.args[0]
    print(message.upper())

d: dict[str, int] = {}
try:
    d["k"]
except KeyError as error:
    print(error.args, error.args[0])
