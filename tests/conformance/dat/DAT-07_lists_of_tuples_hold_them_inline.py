# typy-test: DAT-07
# typy-expect: accepted
pairs = [(2, "b"), (1, "z"), (1, "a")]
print(pairs, len(pairs), pairs[0], pairs[-1][1])
pairs.append((0, "q"))
pairs.sort()
print(pairs, sorted(pairs, reverse=True))
print((1, "a") in pairs, pairs.index((1, "z")), pairs.count((5, "x")))
n, s = pairs.pop()
print(n, s, pairs)
pairs.insert(0, (9, "nine"))
pairs.extend(pairs)
print(pairs)
pairs.remove((9, "nine"))
print(pairs, max(pairs), min(pairs))
print(pairs[1:3], pairs[::-1][0], pairs + [(7, "x")], [(1, "o")] * 2)
for number, text in pairs:
    print(number, text)
pairs[0] = (100, "hundred")
print(pairs[0], pairs == pairs.copy(), pairs < [(100, "z")])
big = [(2 ** 70, "big")]
print(big, big[0][0] + 1)
