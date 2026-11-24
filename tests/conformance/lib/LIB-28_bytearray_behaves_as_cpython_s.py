# typy-test: LIB-28, DAT-06
# typy-expect: accepted
# A bytearray is made, written, compared, searched and changed as CPython does it, its
# methods of bytes giving bytearrays, and `+=` and `*=` changing it in place.
data = bytearray(b"hello")
empty = bytearray()
zeros = bytearray(3)
numbers = bytearray([72, 105])
text = bytearray("caf\xe9", "utf-8")
print(data, empty, zeros, numbers, text, len(data), len(empty))
print(repr(data), str(zeros), bytes(data), bytearray(data))
print(data == b"hello", data == bytearray(b"hello"), b"hello" == data, data != b"x")
print(data < b"z", b"a" < data, 104 in data, b"ell" in data, bytearray(b"ll") in b"hello")
if data:
    print("truth")
data = bytearray(b"hello world")
data.append(33)
data.extend(b" ok")
data.extend(bytearray(b"!"))
data.extend([65, 66])
data.insert(0, 62)
print(data, data.pop(), data.pop(0), data)
data.remove(32)
data.reverse()
copy = data.copy()
data.clear()
print(data, copy, len(copy))
text = bytearray(b"  Alpha,Beta,Gamma  ")
print(text.strip(), text.upper(), text.split(b","), text.strip().split(bytearray(b",")))
print(text.find(b"Beta"), text.count(b"a"), text.startswith(b"  A"), text.replace(b"a", b"4"))
print(text.partition(b","), text.rpartition(bytearray(b",")), text.decode(), text.hex())
print(bytearray(b"x").join([b"a", b"b"]), text.isascii(), bytearray(b"ab").center(6, b"*"))
for word in text.strip().split(b","):
    print(word, type(word).__name__)
data = bytearray(b"abc")
print(data + b"d", data + bytearray(b"e"), b"z" + data, data * 2, 2 * data)
data += b"de"
data += bytearray(b"f")
data *= 2
print(data, data[0], data[-1], data[1:4], data[::-2])
data[0] = 65
data[1:3] = b"XYZ"
data[-1:] = bytearray(b"!")
del data[0]
del data[::3]
print(data, len(data))
total = 0
for value in data:
    total += value
print(total, list(data), sorted(data)[:3], sum(data), max(data))
print(isinstance(data, bytearray), isinstance(data, bytes), type(data).__name__, type(data))
try:
    data[100] = 1
except IndexError as error:
    print("IndexError", error)
try:
    data[0] = 256
except ValueError as error:
    print("ValueError", error)
alias = bytearray(b"ab")
same = alias
alias += b"cd"
alias *= 2
print(same, alias, same == alias.copy())
for index, value in enumerate(bytearray(b"xyz")):
    print(index, value)
words = [bytearray(b"b"), bytearray(b"a")]
words.sort()
print(words, words[0] < words[1], max(words), bytes(words[0]) == b"a")
