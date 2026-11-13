# typy-test: EXC-07, STD-05
# typy-expect: accepted
def check(values: list[int]) -> None:
    try:
        print(values[3])
    except IndexError as error:
        print("logging", error)
        raise


try:
    check([1])
except IndexError as error:
    print("again:", error)
check([1, 2])
