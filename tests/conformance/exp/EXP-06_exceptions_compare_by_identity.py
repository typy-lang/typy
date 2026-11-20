# typy-test: EXP-06
# typy-expect: accepted
class Problem(Exception):
    pass


first = ValueError("a")
second = ValueError("a")
print(first == second, first == first, first != second)
one = Problem("a")
print(one == Problem("a"), one == one, one != one)
