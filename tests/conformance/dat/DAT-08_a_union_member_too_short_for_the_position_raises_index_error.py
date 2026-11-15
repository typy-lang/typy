# typy-test: DAT-08
# typy-expect: accepted
def pick(flag: int) -> tuple[str] | tuple[str, int]:
    if flag:
        return ("a", 1)
    return ("b",)


def items(flag: int) -> tuple[int, int, int] | list[str]:
    if flag:
        return (1, 2, 3)
    return ["x"]


for flag in [1, 0]:
    value = pick(flag)
    print(value[0], value[-1])
    try:
        print(value[1])
    except IndexError as error:
        print("IndexError:", error)
    try:
        print(value[-2])
    except IndexError as error:
        print("IndexError:", error)
    other = items(flag)
    try:
        print(other[2])
    except IndexError as error:
        print("IndexError:", error)
