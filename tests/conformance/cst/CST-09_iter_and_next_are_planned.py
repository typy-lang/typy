# typy-test: CST-09
# typy-expect: rejected E0201 at 4:7
numbers = [1, 2]
print(next(numbers, 0))
