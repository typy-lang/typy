# typy-test: FUN-14
# typy-expect: rejected E0404 at 5:33
from collections.abc import Callable

parse: Callable[[str], bytes] = int
