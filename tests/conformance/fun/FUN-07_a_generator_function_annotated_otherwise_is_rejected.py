# typy-test: FUN-07
# typy-expect: rejected E0405 at 3:1
def count(n: int) -> list[int]:
    i = 0
    while i < n:
        yield i
        i += 1
