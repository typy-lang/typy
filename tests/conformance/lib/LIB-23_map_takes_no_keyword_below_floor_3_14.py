# typy-test: LIB-23
# typy-expect: rejected E0404 at 3:26
print(list(map(str, [1], strict=True)))
