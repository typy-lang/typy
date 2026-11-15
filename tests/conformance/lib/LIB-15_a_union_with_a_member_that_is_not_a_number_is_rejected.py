# typy-test: LIB-15
# typy-expect: rejected E0404 at 6:17
import math

values: list[int | str] = [1, "a"]
print(math.fsum(values))
