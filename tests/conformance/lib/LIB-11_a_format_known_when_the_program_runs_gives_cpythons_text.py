# typy-test: LIB-11
# typy-expect: accepted
def show(format: str, value: int) -> str:
    return format % value


fmt = "%s has %d items, %5.2f%% done, %-6s| %x %o %#X %c %r %a"
print(fmt % ("cart", 3, 41.5, "ok", 255, 8, 255, 65, "q", "\u00e9"))
print(show("[%05d]", 42), show("%+d", 7), show("%c", 9731), show("%s", 1))
pair = "%s and %s"
print(pair % (1.5, None), pair % (True, [1, 2]))
named = "%(name)s is %(age)d"
person: dict[str, int | str] = {"name": "Ann", "age": 30}
print(named % person)
values: tuple[int, ...] = (1, 2, 3)
print("%d-%d-%d" % values, ("%*d|%-*d|" % (5, 42, 4, 7)), "%.*f" % (2, 3.14159))
message = "total: %s"
message %= 12
print(message, "plain" % ())
cases: list[str] = ["%d", "%x", "%s %s", "%", "%y", "%(a)s", "% 5"]
for case in cases:
    try:
        print(case % 2.5)
    except TypeError:
        print("TypeError")
    except ValueError:
        print("ValueError")
for case in ["abc", "%s"]:
    try:
        print(case % (1, 2))
    except TypeError:
        print("TypeError")
try:
    print("%(missing)s" % {"name": 1})
except KeyError as error:
    print("KeyError", error)
