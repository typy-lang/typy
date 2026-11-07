# typy-test: DAT-01
# typy-expect: accepted
small = 2 ** 62 - 1
print(small, small + 1, -small - 2)
print(2 ** 200, -(2 ** 200) // 7, 2 ** 200 % 1000)
print(10 ** 30 * 10 ** 30 - 1)
