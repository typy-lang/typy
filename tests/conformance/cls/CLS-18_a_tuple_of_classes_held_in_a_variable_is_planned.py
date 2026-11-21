# typy-test: CLS-18
# typy-expect: rejected E0201 at 4:21
classes = (int, str)
print(isinstance(1, classes))
