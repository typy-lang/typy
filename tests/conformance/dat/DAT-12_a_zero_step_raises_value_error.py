# typy-test: DAT-12
# typy-expect: accepted
xs = [1, 2, 3]
step = len(xs) - 3
print(xs[::1])
print(xs[::step])
