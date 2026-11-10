# typy-test: DAT-07
# typy-expect: accepted
nones = [None, None]
print(nones, None in nones, len(nones), nones.count(None), nones == [None, None])
nones.append(None)
print(nones.pop(), nones, any(nones))
