# typy-test: EXP-22
# typy-expect: rejected E0409 at 4:32
codes = [1, 2]
raise ValueError("bad codes", *codes)
