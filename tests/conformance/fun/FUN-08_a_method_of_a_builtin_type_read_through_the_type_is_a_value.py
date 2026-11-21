# typy-test: FUN-08
# typy-expect: accepted
from collections.abc import Callable

words = ["pear", "Fig", "banana", "kiwi"]
print(sorted(words, key=str.lower), max(words, key=str.upper), min(words, key=str.casefold))
shout: Callable[[str], str] = str.upper
add: Callable[[list[int], int], None] = list.append
numbers = [1]
add(numbers, 2)
print(shout("hi"), numbers, shout, add, type(shout))
print(shout is str.upper, shout == str.upper, shout is not str.lower)
