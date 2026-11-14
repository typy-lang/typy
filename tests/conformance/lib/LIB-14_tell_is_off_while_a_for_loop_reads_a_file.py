# typy-test: LIB-14
# typy-expect: accepted
with open("lines.txt", "w") as out:
    out.write("ab\r\ncd\nef\n")
file = open("lines.txt")
for line in file:
    break
for action in ["tell", "seek 0 1", "readline"]:
    try:
        if action == "tell":
            file.tell()
        elif action == "seek 0 1":
            file.seek(0, 1)
        else:
            print(repr(file.readline()))
            file.tell()
    except OSError as error:
        print(action, error)
print(file.seek(0, 2), file.tell(), file.seek(0))
for line in file:
    pass
print(file.tell(), file.seek(0), repr(file.read(3)), file.tell())
print(file.readlines(), file.tell())
file.close()
