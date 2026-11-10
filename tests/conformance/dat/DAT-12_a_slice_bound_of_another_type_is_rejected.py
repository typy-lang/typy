# typy-test: DAT-12
# typy-expect: rejected E0404 at 4:10
xs = [1, 2, 3]
print(xs[1.5:])
