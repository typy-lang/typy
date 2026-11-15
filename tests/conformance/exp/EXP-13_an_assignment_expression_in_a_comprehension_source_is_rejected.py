# typy-test: EXP-13
# typy-expect: rejected E0103 at 4:14
x = [1]
[1 for y in (z := x)]
