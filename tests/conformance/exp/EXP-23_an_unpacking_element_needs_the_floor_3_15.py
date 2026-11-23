# typy-test: EXP-23
# typy-expect: rejected E0103 at 4:8
# Below the Python floor 3.15, an element that unpacks is CPython's syntax error.
print([*row for row in [[1, 2], [3]]])
