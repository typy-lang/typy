# typy-test: LIB-14
# typy-expect: accepted
def content() -> str:
    with open("p.txt", newline="") as f:
        return f.read()


with open("p.txt", "w") as out:
    out.write("abcdef\nghi\n")
f = open("p.txt", "r+")
print(f.mode, f.readable(), f.writable(), repr(f))
print(repr(f.read(2)), f.tell(), f.write("X"), f.tell())
f.flush()
print(repr(content()))
print(repr(f.read()), f.seek(0), f.write("YY"), repr(f.read()))
f.close()
print(repr(content()))
f = open("p.txt", "w+")
print(repr(f.read()), f.write("hello\n"), repr(f.read()), f.tell(), f.seek(0), repr(f.read()))
f.close()
with open("p.txt", "w") as out:
    out.write("base\n")
f = open("p.txt", "a+")
print(f.tell(), repr(f.read()), f.seek(0), repr(f.readline()), f.write("more\n"), f.tell())
f.seek(0)
print(repr(f.read()))
f.close()
try:
    open("p.txt", "x+")
except FileExistsError as error:
    print(error)
try:
    open("missing.txt", "r+")
except FileNotFoundError as error:
    print(error)
f = open("p.txt", "r+")
print(f.truncate(1), repr(f.read()))
f.close()
with open("p.txt", "w") as out:
    out.write("line1\nline2\nline3\n")
f = open("p.txt", "r+")
for line in f:
    print(repr(line), f.write("Z"))
    break
f.close()
print(repr(content()))
f = open("p.txt", "r+")
f.readline()
print(f.tell(), f.seek(6), f.write("Q"))
f.close()
print(repr(content()))
