# typy-test: LIB-03
# typy-expect: accepted
xs: list[int] = []
xs.append(1)
xs.append(2)
print(xs, xs.append(3), xs)
