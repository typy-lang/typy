# typy-test: LIB-04
# typy-expect: accepted
from typing import Iterable


def count_lines(lines: Iterable[str]) -> int:
    total = 0
    for _ in lines:
        total += 1
    return total


with open("notes.txt", "w") as notes:
    print(notes.write("alpha\r\nbeta\n"), notes.write("gamma"))
    notes.writelines(["\rdelta\n", "\u00e9psilon\n"])
    notes.writelines(("tuple", " line\n"))
    print("printed", 1, sep="-", file=notes)
with open("notes.txt") as notes:
    print(repr(notes.readline()), repr(notes.read(3)), repr(notes.readline(2)))
    for line in notes:
        print(repr(line))
    print(repr(notes.read()), repr(notes.readline()))
print(open("notes.txt").readlines())
print(list(open("notes.txt", "rt")))
print([line.strip() for line in open("notes.txt", encoding="utf-8")])
print(count_lines(open("notes.txt", encoding="UTF8")))
lines = open("notes.txt")
print(repr(next(lines)), repr(next(iter(lines))), sorted(lines)[0])
lines.close()
with open("log.txt", "a") as log:
    log.write("first\n")
with open("log.txt", mode="a") as log:
    log.write("second\n")
with open("log.txt", "r") as log:
    print(repr(log.read()))
with open("new.txt", "x") as created:
    created.write("made once\n")
print(open("new.txt").read(), end="")
empty = open("empty.txt", "w")
empty.close()
empty.close()
print(repr(open("empty.txt").read()), open("empty.txt").readlines())
for number, line in enumerate(open("notes.txt"), 1):
    print(number, line.rstrip())
for first, second in zip(open("notes.txt"), open("log.txt")):
    print(first.strip(), second.strip())
print({line.strip(): len(line) for line in open("log.txt")})
