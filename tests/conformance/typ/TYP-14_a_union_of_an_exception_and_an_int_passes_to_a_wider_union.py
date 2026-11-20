# typy-test: TYP-14
# typy-expect: accepted
def describe(problem: Exception | int) -> str:
    return str(problem)


def check(problem: ValueError | int) -> str:
    return describe(problem)


print(check(ValueError("bad")), check(3))
