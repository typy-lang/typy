# typy-test: EXP-14, EXP-12
# typy-expect: accepted
from typing import Callable

i = 100
made = [lambda: i for i in range(3)]
show: Callable[[], int] = lambda: i
pairs = {k: (lambda: k * 2) for k in range(2)}
print([each() for each in made], show(), [f() for f in pairs.values()])
i = 7
print(show())
