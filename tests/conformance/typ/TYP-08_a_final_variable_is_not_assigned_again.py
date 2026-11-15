# typy-test: TYP-08
# typy-expect: rejected E0414 at 10:5
from typing import Final

LIMIT: Final = 10


def raise_limit() -> None:
    global LIMIT
    LIMIT = 11
