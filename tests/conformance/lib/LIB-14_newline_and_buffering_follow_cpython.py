# typy-test: LIB-14
# typy-expect: accepted
with open("lines.txt", "w", newline="") as out:
    out.write("ab\r\r\ncd\r")
for newline in [None, "", "\n", "\r", "\r\n"]:
    file = open("lines.txt", newline=newline)
    print(repr(newline), file.readlines())
    file.close()
    file = open("lines.txt", newline=newline)
    print(repr(file.readline(3)), repr(file.read(2)), repr(file.read()))
    file.close()
    out = open("written.txt", "w", newline=newline)
    out.write("x\ny\r\nz\r")
    print(out.tell())
    out.close()
    print(repr(open("written.txt", newline="").read()))
out = open("written.txt", "w", buffering=1)
out.write("ab")
print(repr(open("written.txt").read()))
out.write("c\nd")
print(repr(open("written.txt").read()))
out.write("e\r")
print(repr(open("written.txt", newline="").read()))
out.close()
for buffering in [0, 1 << 40, -5, 4096]:
    try:
        open("written.txt", "w", buffering=buffering).close()
        print("opened", buffering)
    except ValueError as error:
        print("ValueError", error)
    except OverflowError as error:
        print("OverflowError", error)
for illegal in ["x", "\n\n"]:
    try:
        open("written.txt", newline=illegal)
    except ValueError as error:
        print("ValueError", repr(str(error)))
