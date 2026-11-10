# typy-test: DAT-11
# typy-expect: accepted
xs = [10, 20, 30]
print(xs[0], xs[2], xs[-1], xs[-3])
xs[-1] = 33
print(xs)
print(xs[3])
