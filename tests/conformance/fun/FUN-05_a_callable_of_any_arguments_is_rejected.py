# typy-test: FUN-05
# typy-expect: rejected E0408 at 5:19
from typing import Callable

handler: Callable[..., int]
