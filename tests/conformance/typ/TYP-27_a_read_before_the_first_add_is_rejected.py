# typy-test: TYP-27
# typy-expect: rejected E0410 at 4:13
def largest(values: list[int]) -> int:
    found = []
    print(len(found))
    found.extend(values)
    return max(found)


print(largest([3, 1, 2]))
