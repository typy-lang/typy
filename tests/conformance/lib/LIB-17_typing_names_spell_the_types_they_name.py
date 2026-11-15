# typy-test: LIB-17
# typy-expect: accepted
import typing
from typing import Optional, Union, List, Dict, Tuple, Set, Type


class Shape:
    pass


def first(values: List[int]) -> Optional[int]:
    return values[0] if values else None


def total(table: Dict[str, int], keys: Tuple[str, ...]) -> Union[int, float]:
    if not keys:
        return 0.5
    return sum([table[key] for key in keys])


def name_of(kind: Type[Shape]) -> str:
    return kind.__name__


def pick(flag: bool) -> typing.Union[str, None, int]:
    return "yes" if flag else None


seen: Set[int] = {1}
pairs: typing.List[typing.Tuple[int, str]] = [(1, "a")]
maybe: typing.Optional[typing.Dict[str, typing.List[int]]] = None
print(first([3]), first([]), total({"a": 1}, ("a",)), total({}, ()), name_of(Shape))
print(seen, pairs, maybe, pick(True), pick(False))
