# typy-test: EXC-15
# typy-expect: accepted
def parse(text: str) -> int:
    if not text:
        raise ValueError("empty", 0)
    return int(text)


for text in ["", "x"]:
    try:
        parse(text)
    except ValueError as error:
        if len(error.args) == 2:
            count: int = error.args[1]
            print(count + 1)
        print(sorted(error.args[:1]))
try:
    raise OSError(2, "gone", None)
except FileNotFoundError as error:
    print(error.args, error.args[2])
