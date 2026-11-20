# typy-test: EXP-19
# typy-expect: rejected E0201 at 9:9
def pair(a: int, b: int) -> int:
    return a + b


values = [1, 2]
f = pair
print(f(*values))
