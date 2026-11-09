# typy-test: LIB-02
# typy-expect: accepted
values = [0.5, 1.5, 2.5, -0.5, -2.5, 2.675, 1.005, 0.125, 0.375, 123.456, -123.456,
          1e300, 5e-324, 2.2250738585072014e-308, 0.1, 1234567.891, 2.5e-5, -0.0, 0.4,
          -0.4, 99.95, 9.995, 1e16, 4503599627370497.0, 0.049999999999999996,
          float("inf"), float("nan")]
places = [0, 1, 2, 3, -1, -2, -3, 5, 10, 17, 300, 323, 324, -308, -309, -400, 2 ** 70,
          -(2 ** 70)]
for value in values:
    for digits in places:
        print(repr(round(value, digits)), end=" ")
    print()
for value in values[:-2]:
    print(round(value), end=" ")
print()
for number in [0, 5, 15, 25, -25, 149, 150, 151, 2 ** 70 + 5, -(10 ** 30) - 5 * 10 ** 29]:
    print(round(number), round(number, 1), round(number, -1), round(number, -2),
          round(number, -30), round(number, ndigits=-40))
print(round(1.7976931348623157e308, -308))
