# typy-test: LIB-14
# typy-expect: accepted
out = open("b.data", "wb")
print(repr(out), out.mode, out.readable(), out.writable(), out.seekable())
print(out.write(b"ab\r\ncd\ref\n\xff"), out.tell(), out.seek(-2, 1), out.seek(-1, 2), out.seek(0, 3))
for offset, whence in [(-100, 1), (-1, 0), (0, 5), (1 << 63, 0)]:
    try:
        out.seek(offset, whence)
    except OSError as error:
        print("OSError", error)
    except ValueError as error:
        print("ValueError", error)
try:
    out.read()
except OSError as error:
    print("read", error)
out.writelines([b"1", b"2"])
print(out.truncate(5), out.tell())
out.close()
print(repr(out), out.closed, out.readable())
for action in ["write", "flush", "seek", "tell", "truncate", "writable", "fileno", "writelines", "read"]:
    try:
        if action == "write":
            out.write(b"x")
        elif action == "flush":
            out.flush()
        elif action == "seek":
            out.seek(0)
        elif action == "tell":
            out.tell()
        elif action == "truncate":
            out.truncate()
        elif action == "writable":
            out.writable()
        elif action == "fileno":
            out.fileno()
        elif action == "writelines":
            out.writelines([b"x"])
        else:
            out.read()
    except ValueError as error:
        print(action, "ValueError", error)
    except OSError as error:
        print(action, "OSError", error)
data = open("b.data", "rb")
print(repr(data), data.mode, repr(data.read(2)), repr(data.readline()), repr(data.readline(2)), data.tell())
print(data.readlines(), repr(data.read()), data.seek(1), repr(data.read(-1)), data.seek(-3, 2), repr(data.read(None)))
data.seek(0)
print([line for line in data], data.tell())
for action in ["write", "truncate"]:
    try:
        if action == "write":
            data.write(b"x")
        else:
            data.truncate()
    except OSError as error:
        print(action, error)
data.close()
for action in ["read", "readline", "readlines", "iter", "readable", "writable"]:
    try:
        if action == "read":
            data.read()
        elif action == "readline":
            data.readline()
        elif action == "readlines":
            data.readlines()
        elif action == "iter":
            for line in data:
                pass
        elif action == "readable":
            data.readable()
        else:
            print("writable", data.writable())
    except ValueError as error:
        print(action, error)
with open("d.data", "wb") as out:
    out.write(b"abcdef\nghi\n")
both = open("d.data", "r+b")
print(repr(both), repr(both.read(2)), both.write(b"X"), both.tell(), repr(both.read()))
both.close()
print(open("d.data", "rb").read())
both = open("d.data", "a+b")
print(both.tell(), both.seek(0), repr(both.read(1)), both.write(b"W"), both.tell())
both.close()
print(open("d.data", "rb").read())
raw = open("d.data", "rb", buffering=0)
print(repr(raw), raw.mode, repr(raw.read(1)), repr(raw.readline()), raw.tell(), raw.readlines())
try:
    raw.write(b"x")
except OSError as error:
    print(error)
try:
    raw.seek(0, 5)
except OSError as error:
    print(error)
raw.close()
print(repr(raw))
try:
    raw.read()
except ValueError as error:
    print(error)
raw = open("d.data", "ab", buffering=0)
print(repr(raw), raw.write(b"zz"), raw.tell())
raw.close()
print(open("d.data", "rb").read())
