# typy-test: EXC-13
# typy-expect: accepted
codes: list[int | str | None] = [None, 3, "message"]
for code in codes:
    try:
        raise SystemExit(code)
    except SystemExit as stop:
        print(stop.code, repr(stop.code), stop.code == 3, stop.code == "message", not stop.code)
try:
    raise SystemExit
except SystemExit as stop:
    print(stop.code, stop.code == None, bool(stop.code), f"{stop.code!r}")
try:
    raise SystemExit(1, 2)
except SystemExit as stop:
    print(stop.code, stop.code == (1, 2), stop.args)
maybe: int | None = None
other: int | str = 3
values: list[int | None] = [maybe, 3]
for given in values:
    try:
        raise SystemExit(given)
    except SystemExit as stop:
        print(stop.code == maybe, stop.code == other, stop.code != other)
