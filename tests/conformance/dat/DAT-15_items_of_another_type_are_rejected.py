# typy-test: DAT-15
# typy-expect: rejected E0404 at 4:15
values = set([1])
values.update(("a", "b"))
