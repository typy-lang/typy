# typy-test: EXP-22
# typy-expect: rejected E0201 at 9:11
def pair(a: int, b: int) -> int:
    return a + b


values = [1, 2]
f = pair
print(max(*values))
