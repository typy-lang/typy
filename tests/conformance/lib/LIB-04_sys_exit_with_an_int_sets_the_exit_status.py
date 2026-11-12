# typy-test: LIB-04
# typy-expect: accepted
import sys


def finish(code: int) -> None:
    print("finishing with", code)
    sys.exit(code)


try:
    finish(0)
finally:
    print("cleaned up")
print("not reached")
