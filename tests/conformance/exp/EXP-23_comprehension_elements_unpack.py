# typy-test: EXP-23
# typy-expect: accepted
# typy-python: 3.15
# A comprehension element that unpacks puts each item, or each key and value, of each
# iterable or mapping in the result, as PEP 798 defines it.
rows = [[1, 2], [3], []]
words = ["ab", "c"]
tables = [{"a": 1}, {"b": 2, "a": 3}]

print([*row for row in rows], sorted({*row for row in rows}), list(*row for row in rows))
print([*word for word in words], {**table for table in tables})
print([*range(n) for n in range(4) if n % 2], sum(*row for row in rows))
print([[*cell for cell in grid] for grid in [[[1], [2, 3]], [[4]]]])
