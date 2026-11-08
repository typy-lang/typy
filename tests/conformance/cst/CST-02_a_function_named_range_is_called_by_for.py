# typy-test: CST-02
# typy-expect: accepted
def range(n: int) -> list[int]:
    return [n, n]


for i in range(3):
    print(i)
