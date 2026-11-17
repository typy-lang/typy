# typy-test: STD-04
# typy-expect: rejected E0202 at 3:19
from first import run


def run() -> int:
    return 3
