# typy-test: LIB-28
# typy-expect: rejected E0201 at 3:18
quotient, rest = divmod(17, 5)
print(quotient, rest)
