# typy-test: DAT-18
# typy-expect: rejected E0401 at 6:1
# typy-python: 3.15
# A frozendict never changes, so CPython raises TypeError for an item assignment.
prices: frozendict[str, int] = frozendict({"tea": 3})
prices["cake"] = 5
