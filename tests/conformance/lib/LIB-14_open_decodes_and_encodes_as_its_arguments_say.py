# typy-test: LIB-14
# typy-expect: accepted
with open("coded.txt", "w", encoding="latin-1", newline="") as out:
    out.write("a\u00e9b\r\nc\rd\ne")
files = [
    open("coded.txt", encoding="ascii", errors="replace"),
    open("coded.txt", encoding="ascii", errors="ignore"),
    open("coded.txt", encoding="ascii", errors="backslashreplace"),
    open("coded.txt", encoding="latin-1"),
    open("coded.txt", encoding="utf-8", errors="replace"),
    open("coded.txt", encoding="Latin1", errors=None),
]
for file in files:
    print(repr(file.read()), repr(file))
    file.close()
try:
    open("coded.txt", encoding="ascii").read()
except UnicodeDecodeError as error:
    print(error)
try:
    open("coded.txt", errors="xmlcharrefreplace").read()
except TypeError as error:
    print(error)
escaped = open("coded.txt", encoding="ascii", errors="backslashreplace")
print(repr(escaped.read(3)), repr(escaped.read(2)), repr(escaped.read()))
escaped.close()
out = open("coded.txt", "w", encoding="ascii", newline="\r\n")
try:
    out.write("a\n\u00e9")
except UnicodeEncodeError as error:
    print(error)
out.close()
out = open("coded.txt", "w", encoding="ascii", errors="xmlcharrefreplace")
print(out.write("ab\u00e9c\u20ac"))
out.close()
print(open("coded.txt").read())
