# typy-test: DAT-02
# typy-expect: accepted
print(0.1 + 0.2, 1e16, 1e15, 1e-5, 0.0001, -0.0, 2.5, 1e300 * 10.0, 5e-324)
print(str(1.5), repr(1 / 3), float("1e-7"), 100.0)
