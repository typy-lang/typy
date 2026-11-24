# typy-test: LIB-34
# typy-expect: rejected E0413 at 4:7
# A list has no hash, so hash of one always raises TypeError in CPython.
print(hash([1, 2]))
