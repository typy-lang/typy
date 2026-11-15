# typy-test: LIB-14
# typy-expect: accepted
import sys

written = open("state.txt", "w")
print(written.name, written.mode, written.closed, written.fileno() > 2)
print(written.readable(), written.writable(), written.seekable(), written.isatty())
written.close()
print(written.name, written.mode, written.closed, written.readable())
for question in ["fileno", "isatty", "writable", "seekable"]:
    try:
        if question == "fileno":
            written.fileno()
        elif question == "isatty":
            written.isatty()
        elif question == "writable":
            written.writable()
        else:
            written.seekable()
    except ValueError as error:
        print(question, error)
reading = open("state.txt", "rt")
print(reading.mode, reading.readable(), reading.writable())
reading.close()
print(reading.writable())
try:
    reading.readable()
except ValueError as error:
    print("readable", error)
for stream in [sys.stdin, sys.stdout, sys.stderr]:
    print(stream.name, stream.mode, stream.fileno(), stream.closed)
    print(stream.readable(), stream.writable())
