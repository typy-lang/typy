# typy-test: EXP-16
# typy-expect: accepted
def join(first: str, second: str = "b", sep: str = "-") -> str:
    return first + sep + second


print(join("a"), join("a", "c"), join("a", sep="+"), join(second="z", first="y"))
