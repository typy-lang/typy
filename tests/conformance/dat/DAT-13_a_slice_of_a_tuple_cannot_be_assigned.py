# typy-test: DAT-13
# typy-expect: rejected E0401 at 4:1
values: tuple[int, ...] = (1, 2, 3)
values[0:2] = [5]
