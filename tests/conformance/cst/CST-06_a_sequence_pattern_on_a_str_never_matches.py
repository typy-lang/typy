# typy-test: CST-06, TYP-34
# typy-expect: accepted
# A sequence pattern never matches a `str`, as CPython reads it, so its case never runs.
word = "ab"
match word:
    case [first, *rest]:
        print(first, rest)
    case _:
        print("not a sequence pattern's")
