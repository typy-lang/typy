# typy-test: FUN-19
# typy-expect: rejected E0203 at 5:37
from collections.abc import Callable

count: Callable[[], int] = lambda **options: len(options)
