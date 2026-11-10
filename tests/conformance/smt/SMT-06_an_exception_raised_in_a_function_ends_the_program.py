# typy-test: SMT-06, STD-05
# typy-expect: accepted
def checked(value: int) -> int:
    if value < 0:
        raise ValueError("negative value", value)
    return value * 2


print(checked(4))
print(checked(-3))
print("not reached")
