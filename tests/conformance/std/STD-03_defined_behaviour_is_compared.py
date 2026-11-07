# typy-test: STD-03
# typy-expect: accepted
def depth(n: int) -> int:
    if n == 0:
        return 0
    return 1 + depth(n - 1)


print(depth(400))
