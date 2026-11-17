# typy-test: SMT-12
# typy-expect: rejected E0408 at 4:8
from shapes import Pair
value: Pair[int, str] = (1, "a")
