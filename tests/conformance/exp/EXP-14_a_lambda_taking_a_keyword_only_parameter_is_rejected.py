# typy-test: EXP-14, FUN-05
# typy-expect: rejected E0202 at 5:41
from typing import Callable

scale: Callable[[int], int] = lambda *, by: by * 2
