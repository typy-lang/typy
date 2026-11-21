# typy-test: LIB-23
# typy-expect: accepted
# typy-python: 3.14
def attempt(values: list[int], others: list[int], third: list[int]) -> None:
    try:
        print(list(map(lambda a, b, c: a + b + c, values, others, third, strict=True)))
    except ValueError as error:
        print("ValueError:", error)


attempt([1, 2], [3, 4], [5, 6])
attempt([1, 2, 3], [3, 4], [5, 6])
attempt([1, 2], [3, 4, 5], [5, 6])
attempt([1, 2], [3, 4], [5, 6, 7])
attempt([1, 2], [3, 4], [5])
print(list(map(lambda a, b: a * b, [1, 2], [3, 4], strict=False)))
print(list(map(str, [1, 2], strict=True)))
