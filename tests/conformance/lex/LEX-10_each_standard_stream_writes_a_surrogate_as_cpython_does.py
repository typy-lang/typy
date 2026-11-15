# typy-test: LEX-10
# typy-expect: accepted
import sys
print("a", chr(0xdcfe), "b")
try:
    print("x", chr(0xdc80) + chr(0xdfff), "y")
except UnicodeEncodeError as error:
    print("print:", error)
try:
    sys.stdout.write("w" + chr(0xd800))
except UnicodeEncodeError as error:
    print("write:", error)
print("err", chr(0xd800), file=sys.stderr)
sys.stderr.write("text " + chr(0xdc80) + "\n")
raise ValueError("bad " + chr(0xd801))
