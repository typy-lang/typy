# typy-test: EXP-12
# typy-expect: accepted
g = [w for w in range(2) for w in "ab"]
print(g)
w = 5
print([w for w in [w, w + 1]], w)
