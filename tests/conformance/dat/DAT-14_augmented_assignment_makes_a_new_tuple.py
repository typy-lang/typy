# typy-test: DAT-14, SMT-01
# typy-expect: accepted
t: tuple[int, ...] = (1, 2)
u = t
t += (3,)
print(t, u)
v: tuple[int, ...] = (4,)
w = v
v *= 2
print(v, w)
print(v + (5, 6), (0,) + v, len(t + ()))
