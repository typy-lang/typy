# typy-test: STD-06
# typy-expect: rejected E0306 at 5:1
def go() -> int:
    return run()
print(go())
from first import run
