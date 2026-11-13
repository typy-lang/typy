# typy-test: CST-09
# typy-expect: rejected E0404 at 4:19
items = iter([1, 2])
print(next(items, "none"))
