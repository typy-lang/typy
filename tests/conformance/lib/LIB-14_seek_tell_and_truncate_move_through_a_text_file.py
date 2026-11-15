# typy-test: LIB-14
# typy-expect: accepted
file = open("moved.txt", "w")
file.write("h\u00e9llo\nworld\n")
print(file.tell(), file.seek(0), file.write("J"), file.seek(0, 2), file.seek(0, 1))
for cookie, whence in [(1, 1), (-1, 2), (0, 3), (-1, 0), (1 << 63, 0), (0, 1 << 40)]:
    try:
        file.seek(cookie, whence)
    except OSError as error:
        print(type(error).__name__, error)
    except ValueError as error:
        print("ValueError", error)
    except OverflowError as error:
        print("OverflowError", error)
print(file.truncate(3), file.tell(), file.truncate(), file.truncate(None))
for size in [-1, 1 << 70]:
    try:
        file.truncate(size)
    except OSError as error:
        print("OSError", error)
    except OverflowError as error:
        print("OverflowError", error)
file.close()
for action in ["tell", "seek", "truncate"]:
    try:
        if action == "tell":
            file.tell()
        elif action == "seek":
            file.seek(0)
        else:
            file.truncate()
    except ValueError as error:
        print(action, error)
reading = open("moved.txt")
print(reading.read(2), reading.tell(), repr(reading.readline()), reading.tell())
print(reading.seek(1), repr(reading.read()), reading.tell())
try:
    reading.truncate(1)
except OSError as error:
    print("truncate", error)
reading.close()
appending = open("moved.txt", "a")
print(appending.tell(), appending.seek(0), appending.write("Q"), appending.tell())
appending.close()
with open("moved.txt") as again:
    print(repr(again.read()))
