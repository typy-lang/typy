# typy-test: TYP-17
# typy-expect: rejected E0201 at 5:23
from collections.abc import Iterable

print(isinstance([1], Iterable))
