# typy-test: DAT-14
# typy-expect: rejected E0402 at 3:22
t: tuple[int, ...] = (1, "a")
