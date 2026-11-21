# typy-test: FUN-08
# typy-expect: accepted
from collections.abc import Callable

words: list[str] = []
add: Callable[[str], None] = words.append
add("pear")
add("fig")
join: Callable[[list[str]], str] = "-".join
lookup: Callable[[str], int | None] = {"a": 1}.get
print(words, join(words), lookup("a"), lookup("z"))
print(sorted(["b", "a"], key="ab".index), max(["x", "yy"], key=len))
again: Callable[[str], None] = words.append
print(add == again, add is again, type(add), repr(add).startswith("<built-in method append of list object at 0x"))
