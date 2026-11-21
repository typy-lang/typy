# typy-test: TYP-18
# typy-expect: accepted
import collections.abc
import typing
from collections.abc import Callable, Iterator
from typing import Dict, List, Optional, Sequence, Union


class Box:
    pass


def echo[T](value: T) -> T:
    return value


kinds = [list[int], dict[str, list[int]], tuple[int, ...], tuple[()], int | None, Box | None]
print(kinds)
print(List[int], Dict[str, None], typing.Iterator[int], collections.abc.Iterator[int])
print(Callable[[int], str], typing.Callable[..., int], Sequence[Box], type[Box])
print(Iterator, typing.List, collections.abc.Sequence, typing.Protocol, typing.Optional)
print(list[int] == list[int], List[int] == list[int], (int | str) == (str | int))
print(Optional[int] == int | None, Union[str, int] == (int | str), int | int)
print(Iterator is Iterator, typing.List is List, {list[int]: 1}[list[int]])
print(f"{list[int | None]} and {repr(int | str)} and {str(Sequence)}", "%s" % (set[int],))
print(list[int] in kinds, Box | None in kinds, echo(dict[str, int]), kinds.index(int | None))
print(isinstance(3, int | str), isinstance(None, int | None), isinstance("a", Box | float))
