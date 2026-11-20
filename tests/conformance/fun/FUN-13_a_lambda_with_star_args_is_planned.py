# typy-test: FUN-13
# typy-expect: rejected E0201 at 5:35
from typing import Callable

count: Callable[[], int] = lambda *values: 0
