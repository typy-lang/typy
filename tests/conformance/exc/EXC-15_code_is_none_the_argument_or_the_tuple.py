# typy-test: EXC-15
# typy-expect: accepted
import sys


def stop(kind: int) -> None:
    if kind == 0:
        sys.exit()
    if kind == 1:
        sys.exit(3)
    if kind == 2:
        sys.exit("bye")
    raise SystemExit(1, "two")


for kind in range(4):
    try:
        stop(kind)
    except SystemExit as error:
        code = error.code
        print(repr(code), code is None)
        if isinstance(code, int):
            print(code + 1)
