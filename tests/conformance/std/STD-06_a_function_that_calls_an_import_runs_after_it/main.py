# typy-test: STD-06
# typy-expect: accepted
def go() -> int:
    return run()
from first import run
print(go())
