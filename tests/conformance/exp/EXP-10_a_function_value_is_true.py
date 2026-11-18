# typy-test: EXP-10
# typy-expect: accepted
from typing import Callable


def check() -> bool:
    return False


chosen: Callable[[], bool] | None = None
print("unset" if not chosen else "set")
chosen = check
if check:
    print("a function is true, whatever it returns")
print(bool(check), not check, "set" if chosen else "unset", chosen())
