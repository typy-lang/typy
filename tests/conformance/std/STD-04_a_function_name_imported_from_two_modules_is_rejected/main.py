# typy-test: STD-04
# typy-expect: rejected E0202 at 5:20
from first import run
print(run())
from second import run
print(run())
