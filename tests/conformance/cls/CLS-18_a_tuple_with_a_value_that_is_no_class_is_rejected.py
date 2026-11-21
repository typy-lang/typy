# typy-test: CLS-18
# typy-expect: rejected E0401 at 4:21
kinds = (int, 3)
print(isinstance(1, kinds))
