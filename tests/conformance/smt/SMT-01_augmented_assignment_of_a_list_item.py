# typy-test: SMT-01
# typy-expect: accepted
def shrink(xs: list[int]) -> int:
    xs.clear()
    return 1


counts = [1, 2, 3]
counts[0] += 10
counts[-1] *= 4
print(counts)
nested = [[1], [2]]
inner = nested[1]
nested[1] += [5]
print(nested, inner)
words = ["a"]
words[0] *= 3
print(words)
counts[0] += shrink(counts)
print(counts)
