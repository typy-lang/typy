# typy-test: TYP-24
# typy-expect: rejected E0404 at 5:14
from typing import cast
value: object = [1, 2]
items = cast(list[int], value)
