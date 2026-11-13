# typy-test: LIB-04
# typy-expect: accepted
import sys
from sys import argv, stdout, stderr, exit

print(sys.argv[1:], len(argv) > 0)
print(sys.stdout, type(sys.stderr).__name__, stdout is sys.stdout)
count = sys.stdout.write("written\n")
print(count, stderr.write("to standard error\n"), stdout.write("\u00e9\n"))
print("a", "b", sep="-", file=sys.stderr)
print("flushed", file=sys.stdout, flush=True)
print("no file", file=None)
sys.stdout.flush()
try:
    sys.exit(3)
except SystemExit:
    print("caught SystemExit")
try:
    raise SystemExit("message")
except Exception:
    print("not reached")
except BaseException as error:
    print("BaseException", error, repr(error))
try:
    print(input("prompt: "))
except EOFError as error:
    print("EOFError", error)
argv.append("more")
print(sys.argv[-1], argv is not None)
exit("bye")
print("not reached")
