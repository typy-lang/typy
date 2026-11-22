# typy-test: EXP-22
# typy-expect: rejected E0201 at 4:16
rows = [[1, 2], [3, 4]]
print(list(zip(*rows)))
