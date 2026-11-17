# typy-test: EXC-11
# typy-expect: accepted
points: dict[tuple[str, int], float] = {("a", 1): 0.5}
names: dict[str, int] = {}
seen: set[int] = set()
try:
    print(points[("b", 2)])
except KeyError as error:
    print(error, repr(error), error.args)
try:
    del names["gone"]
except KeyError as error:
    print(error, repr(error), error.args)
try:
    names.pop("x")
except KeyError as error:
    print(error, error.args, len(error.args))
try:
    seen.remove(7)
except KeyError as error:
    print(error, error.args)
try:
    seen.pop()
except KeyError as error:
    print(error, repr(error), error.args)
try:
    names.popitem()
except KeyError as error:
    print(error, error.args)
try:
    print("%(size)d" % names)
except KeyError as error:
    print(error, error.args)
