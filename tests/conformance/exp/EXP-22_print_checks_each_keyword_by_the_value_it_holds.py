# typy-test: EXP-22
# typy-expect: accepted
import sys

options = {"file": sys.stdout, "end": None}
print("to stdout", **options)
mixed: dict[str, str | int] = {"sep": "-"}
print(1, 2, **mixed)
numbers: dict[str, int | None] = {"sep": None, "end": None}
print(1, 2, **numbers)
for keyword in ["sep", "end", "file"]:
    counts: dict[str, int | None] = {keyword: 3}
    try:
        print(1, **counts)
    except (TypeError, AttributeError) as error:
        print(type(error).__name__, error)
texts: dict[str, str | None] = {"file": None, "sep": "+"}
print(1, 2, **texts)
with open("print.data", "wb") as binary:
    files = {"file": binary}
    try:
        print("x", **files)
    except TypeError as error:
        print(error)
