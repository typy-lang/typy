# typy-test: LIB-28
# typy-expect: accepted
# `divmod`, `pow`, `bin`, `hex`, `oct` and `format` give CPython's results, on inline and
# large integers, floats and booleans; division by zero prints only the exception's class,
# whose message differs between oracles.
NUMBERS = [0, 1, -1, 7, -7, 2**62, -(2**63), 2**100 + 3]

for a in NUMBERS:
    print(bin(a), hex(a), oct(a), format(a), format(a, "+,d"), format(a, "#x"))
    for b in NUMBERS:
        try:
            print(a, b, divmod(a, b))
        except ZeroDivisionError as error:
            print(a, b, type(error).__name__)

for x, y in [(7.5, 2.0), (-7.5, 2.0), (7.0, -2.5), (1e308, 3.0), (5.0, 0.0)]:
    try:
        print(divmod(x, y))
    except ZeroDivisionError as error:
        print(type(error).__name__)

print(divmod(True, 2), divmod(7, -2.5), divmod(-7.5, 2), bin(True), hex(False), oct(True))
print(pow(2, 10), pow(2.0, -2), pow(-8, 3), pow(base=3, exp=4), pow(2, 3, mod=None))
cases = [(3, -1, 7), (2, 3, -5), (-2, 3, 5), (5, 0, 1), (0, 0, 3), (7, -3, -11), (2**100, 2**100 + 1, 10**20 + 7)]
for base, exponent, modulus in cases:
    print(pow(base, exponent, modulus), pow(base=base, exp=exponent, mod=modulus))
for base, exponent, modulus in [(2, 10, 0), (2, -1, 4)]:
    try:
        print(pow(base, exponent, modulus))
    except ValueError as error:
        print("ValueError", error)

print(format(3.14159, ".2f"), format(1234567.891, ",.1f"), format(0.5, "%"), format(255, "08b"))
print(format("text", ">8"), format("text", "*^9"), format(True), format(True, "d"), format(None))
print(format([1, 2]), format((1, "a")), format(2.5e-7), format(10**20, "_"))
try:
    print(format([1], "x"))
except TypeError as error:
    print("TypeError", error)
try:
    print(format(1, "s"))
except ValueError as error:
    print("ValueError", error)
