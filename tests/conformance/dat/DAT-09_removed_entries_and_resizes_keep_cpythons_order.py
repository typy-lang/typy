# typy-test: DAT-09
# typy-expect: accepted
d: dict[int, int] = {}
for i in range(5000):
    d[i * 7919 % 10007] = i
for i in range(0, 5000, 3):
    d.pop(i * 7919 % 10007)
print(len(d), list(d)[:6], list(d.items())[-3:], sum(d.values()))
while len(d) > 4:
    d.popitem()
print(d, d.copy())
f = {0.0: "zero", 1.5: "x"}
f[-0.0] = "negative zero"
print(f, f[0.0], -0.0 in f)
t = {(1, "a"): [1], (2, "b"): [2]}
t[(1, "a")].append(9)
print(t, t.get((3, "c"), []), (2, "b") in t)
