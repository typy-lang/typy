# typy-test: EXC-15
# typy-expect: accepted
class Parse(ValueError):
    def __init__(self, line: int, text: str) -> None:
        super().__init__(f"line {line}: {text}")
        self.line = line


def show(parts: tuple[str]) -> str:
    return parts[0]


def run(text: str) -> int:
    if not text:
        raise Parse(3, "empty")
    return int(text)


for text in ["", "x", "5"]:
    try:
        print(run(text))
    except Parse as error:
        print("parse", error.args, error.line)
    except ValueError as error:
        kept = error.args
        print("value", kept, type(kept))
        try:
            print(kept[1])
        except IndexError as problem:
            print("index", problem.args)

try:
    raise OSError(2, "gone", "f.txt")
except OSError as error:
    print(error.args, error.args[1])
try:
    assert False, 42
except AssertionError as error:
    print(error.args[0] + 1)
try:
    open("/nonexistent/x")
except FileNotFoundError as error:
    code, text = error.args[0], error.args[1]
    print(code, text)
