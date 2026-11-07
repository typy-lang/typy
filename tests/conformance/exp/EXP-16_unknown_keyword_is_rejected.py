# typy-test: EXP-16
# typy-expect: rejected E0404 at 7:11
def one(n: int) -> int:
    return n


print(one(m=1))
