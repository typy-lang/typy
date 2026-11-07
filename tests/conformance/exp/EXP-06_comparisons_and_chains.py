# typy-test: EXP-06
# typy-expect: accepted
def tick(n: int) -> int:
    print("tick", n)
    return n


print(1 < 2.5, 2 == 2.0, "a" < "b" <= "b", 1 < tick(3) < 2)
print(2 ** 53 + 1 > 2.0 ** 53)
