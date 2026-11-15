# typy-test: EXC-15
# typy-expect: rejected E0404 at 10:17
def first(parts: tuple[str]) -> str:
    return parts[0]


try:
    raise TypeError("bad", 3)
except TypeError as error:
    print(first(error.args))
