# typy-test: LIB-10
# typy-expect: accepted
name = "Ada"
count = 3
ratio = 2.0 / 3
big = 10 ** 25
flag = True
items = [1, 2]
maybe: int | None = None
table = {"x": 1.5, "y": -2.25}
print("%s has %d items" % (name, count))
print("%5.2f|%-8.3e|%+g|%G" % (ratio, ratio, ratio, 1e20))
print("%x %X %#o %08.3d %-6d|" % (255, 255, 8, 5, -3))
print("%d %d %i" % (big, -big, flag))
print("%r %a %s" % (name, "\xe9", items))
print("%c%c%c" % (72, "i", 33))
print("%*d|%-*s|%.*f" % (5, 42, 4, "ab", 2, 3.14159))
print("%(x)6.2f %(y)+.1f %(x)s" % table)
print("100%% of %s" % name)
print("%s" % maybe, "%s" % (maybe,))
print("%s and %s" % ((1, 2), [3]))
print("%d" % 3.99, "%.0f" % 2.5, "%u" % -7)
print("no conversions" % ())
print("%s" % {"a": 1})
print("plain" % {"a": 1})
for value in [0, 1, 1114111]:
    print(repr("%c" % value))
# The messages of `%c` errors differ between CPython versions; the type is the same.
try:
    print("%c" % 1114112)
except OverflowError:
    print("OverflowError")
letters = "ab"
try:
    print("%c" % letters)
except TypeError:
    print("TypeError")
try:
    print("%(z)s" % table)
except KeyError as error:
    print("KeyError", error)
try:
    print("%d" % float("inf"))
except OverflowError as error:
    print("OverflowError", error)
