# typy-test: TYP-10
# typy-expect: accepted
def show(value: int) -> str:
    return repr(value)


print(show(4))
