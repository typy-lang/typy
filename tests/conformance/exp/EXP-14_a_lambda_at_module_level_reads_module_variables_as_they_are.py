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


def comprehension_beside_a_global() -> list[int]:
    global i
    made_here: list[Callable[[], int]] = [lambda: i for i in range(3)]
    return [each() for each in made_here]


print(comprehension_beside_a_global(), i)
