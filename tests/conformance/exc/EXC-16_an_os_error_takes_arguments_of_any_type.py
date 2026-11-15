# typy-test: EXC-16
# typy-expect: accepted
class Quiet(OSError):
    def __init__(self, text: str) -> None:
        pass


class Passing(OSError):
    def __init__(self, code: int, text: str) -> None:
        super().__init__(code, text, "f.txt")


class One(OSError):
    def __init__(self, text: str) -> None:
        super().__init__(text)


def name(flag: bool) -> str | None:
    return "n.txt" if flag else None


for error in [OSError(2, 3), OSError("a", "b"), OSError(None, None), OSError(1, 2, 3, 4, 5, 6),
              OSError(2, "x", None, None, "g"), Quiet("q"), Passing(2, "gone"), One("one"),
              FileNotFoundError(2, "x", 3), OSError(2, "x", name(True), None, "g"),
              OSError(2, "x", name(False), None, "g"), OSError(True, "t")]:
    print(type(error).__name__, repr(error), str(error), error.args, error.errno, error.strerror, error.filename, error.filename2)
print(OSError(2, 3).errno)
try:
    open(99)
except OSError as error:
    print(error, error.args, error.errno, error.filename)
try:
    open("/nonexistent/x")
except OSError as error:
    print(error, error.args, error.errno, error.filename, error.strerror)
