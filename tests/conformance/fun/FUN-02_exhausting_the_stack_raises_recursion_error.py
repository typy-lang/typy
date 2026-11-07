# typy-test: FUN-02
# typy-expect: accepted
def down(n: int) -> int:
    return down(n + 1)


print("start")
print(down(0))
