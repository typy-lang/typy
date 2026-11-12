# typy-test: LIB-04
# typy-expect: accepted
import sys

written = open("written.txt", "w")
written.write("text\n")
try:
    written.read()
except OSError as error:
    print("OSError", error)
try:
    written.readline()
except ValueError as error:
    print("ValueError", error)
written.close()
for action in ["write", "flush", "read", "with"]:
    try:
        if action == "write":
            written.write("more")
        elif action == "flush":
            written.flush()
        elif action == "read":
            written.read()
        else:
            with written:
                pass
    except ValueError as error:
        print(action, error)
reading = open("written.txt")
try:
    reading.write("x")
except OSError as error:
    print(error)
try:
    reading.writelines(["x"])
except ValueError as error:
    print(error)
reading.close()
try:
    for line in reading:
        print(line)
except ValueError as error:
    print("for", error)
try:
    sys.stdout.readline()
except OSError as error:
    print("stdout", error)
try:
    escaped = b"ok \xff\xfe end".decode("utf-8", "surrogateescape")
    open("surrogate.txt", "w").write(escaped)
except UnicodeEncodeError as error:
    print(error)
