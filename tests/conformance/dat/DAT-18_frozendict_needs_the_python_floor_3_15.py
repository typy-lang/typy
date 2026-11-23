# typy-test: DAT-18
# typy-expect: rejected E0425 at 4:10
# frozendict is a builtin from Python 3.15; CPython 3.12 raises NameError.
prices = frozendict({"tea": 3})
print(prices)
