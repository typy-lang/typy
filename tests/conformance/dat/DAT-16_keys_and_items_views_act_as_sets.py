# typy-test: DAT-16
# typy-expect: accepted
big = {n: n * n for n in range(40)}
small = {n: n for n in range(0, 80, 3)}
sevens = set(range(0, 100, 7))
print(big.keys() & small.keys(), small.keys() & big.keys())
print(big.keys() & sevens, sevens & big.keys(), big.keys() & [3, 99, 6])
print(big.keys() | small.keys(), small.keys() | sevens, sevens | big.keys())
print(big.keys() - small.keys(), sevens - big.keys(), big.keys() - [1, 2, 3])
print(big.keys() ^ small.keys(), sevens ^ small.keys(), [1, 100] ^ big.keys())
print(big.items() & small.items(), small.items() & {(0, 0), (3, 3), (5, 5)})
print(big.items() ^ small.items())
print(small.items() ^ big.items())
print(small.items() - big.items(), big.items() | [(99, 0)])
print(small.items() ^ [(0, 0), (1, 2)], {(3, 3), (4, 4)} - small.items())
ages = {"ada": 36, "alan": 41, "grace": 85}
other = {"alan": 41, "grace": 80, "linus": 54}
print(sorted(ages.keys() | other.keys()), sorted(ages.items() ^ other.items()))
