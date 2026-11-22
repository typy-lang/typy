# typy-test: TYP-18
# typy-expect: accepted
# typy-python: 3.14
import typing
from typing import Optional, Union


class Box:
    pass


print(Optional[int], Union[int, str, None], Optional[Box], typing.Union)
print(typing.List[Optional[int]], [Optional[str], Union[int, float]])
held: object = Optional[int]
print(held == int | None)
