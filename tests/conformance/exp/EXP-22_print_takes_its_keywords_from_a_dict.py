# typy-test: EXP-22
# typy-expect: accepted
import sys

texts: list[dict[str, str]] = [{"sep": "-"}, {"end": "!\n"}, {"sep": "", "end": "\n"}, {"file": "x"}]
for options in texts:
    try:
        print("a", "b", **options)
    except AttributeError as error:
        print("AttributeError", error)
numbers = {"sep": 3}
try:
    print("a", **numbers)
except TypeError as error:
    print(error)
extra = {"sep": "+"}
try:
    print("a", sep="-", **extra)
except TypeError as error:
    print(error)
maybe: dict[str, str | None] = {"sep": None, "end": None}
print("a", "b", **maybe)
flags = {"flush": True}
print("c", **flags)
files = {"file": sys.stdout}
print("d", "e", **files)
both: dict[str, str] = {"end": "?\n"}
print(*["x", "y"], sep="+", **both)
