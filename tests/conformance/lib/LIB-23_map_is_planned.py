# typy-test: LIB-23
# typy-expect: rejected E0201 at 3:12
print(list(map(str, [1, 2])))
