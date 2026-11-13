# typy-test: EXC-08
# typy-expect: accepted
values: list[int] = [1, 2]
errors: list[Exception] = [
    ValueError(),
    ValueError("one"),
    ValueError(7),
    ValueError("pair", 2),
    KeyError("key"),
    KeyError(3),
    IndexError(values),
    TypeError(None),
    RuntimeError(1.5, True, (1, "a")),
]
for error in errors:
    print(str(error), repr(error), f"[{error}] {error!r}")
print(errors)
