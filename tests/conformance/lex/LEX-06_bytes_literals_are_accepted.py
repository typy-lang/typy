# typy-test: LEX-06
# typy-expect: accepted
data = b"abc\x00\xff"
raw = rb"\n"
joined = b"one" b"two"
print(data, raw, joined, len(data), data[3], data[-1])
