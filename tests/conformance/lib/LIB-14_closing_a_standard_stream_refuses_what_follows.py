# typy-test: LIB-14
# typy-expect: accepted
import sys

print("before", end="")
sys.stdout.close()
sys.stdout.close()
print(sys.stdout.closed, sys.stdout.readable(), file=sys.stderr)
for action in ["print", "write", "flush", "fileno", "writable", "tell", "with", "writelines", "input"]:
    try:
        if action == "print":
            print("x")
        elif action == "write":
            sys.stdout.write("x")
        elif action == "flush":
            sys.stdout.flush()
        elif action == "fileno":
            sys.stdout.fileno()
        elif action == "writable":
            sys.stdout.writable()
        elif action == "tell":
            sys.stdout.tell()
        elif action == "with":
            with sys.stdout:
                pass
        elif action == "writelines":
            sys.stdout.writelines(["a"])
        else:
            input("prompt")
    except ValueError as error:
        print(action, error, file=sys.stderr)
print(repr(sys.stdout), file=sys.stderr)
sys.stdin.close()
try:
    sys.stdin.readline()
except ValueError as error:
    print("stdin", error, file=sys.stderr)
sys.stderr.close()
try:
    print("lost", file=sys.stderr)
except ValueError as error:
    sys.exit(4)
