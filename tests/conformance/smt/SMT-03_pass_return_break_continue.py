# typy-test: SMT-03
# typy-expect: accepted
def first_odd(limit: int) -> int:
    for n in range(limit):
        if n % 2 == 0:
            continue
        return n
    return -1


while True:
    pass
    break
print(first_odd(10), first_odd(1))
