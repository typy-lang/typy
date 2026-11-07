# typy-test: LIB-01
# typy-expect: accepted
print(len("abc"), abs(-2.5), abs(-3), min(3, 1, 2), max("a", "b"), sep=", ", end="!\n")
print(int("-12"), int(" ff ", 16), int(2.9), float("1.5"), float(3), str(1.5), bool(0), bool("x"))
print(repr("it's"), ord("a"), chr(98), int(), float(), str(), bool())
for i in range(2, 9, 3):
    print(i)
