# typy-test: DAT-09
# typy-expect: rejected E0404 at 4:1
counts = {"a": 1}
counts.setdefault("b")
