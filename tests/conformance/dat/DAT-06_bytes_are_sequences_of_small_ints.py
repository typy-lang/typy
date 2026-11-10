# typy-test: DAT-06
# typy-expect: accepted
data = b"hello\x00\xff"
empty: bytes = bytes()
print(data, len(data), data[0], data[-1], data[1:3], data[::-1], data + b"!", data * 2, 2 * b"ab")
print(b"it's", b'a"b', b"'\"", repr(data), str(b"x"), empty, bytes(3), bytes([65, 66, 255]))
print(104 in data, 7 in data, b"ell" in data, b"" in data, b"z" not in data)
print(data == b"hello\x00\xff", b"a" < b"b", b"ab" < b"a", b"\x80" > b"\x7f", bool(b""), bool(b"\x00"))
for byte in b"AZ":
    print(byte)
print(list(b"hi"), sorted(b"cba"), sum(b"\x01\x02"), max(b"az"), bytes(reversed(b"ab")), [b"x", b"y"])
values: list[bytes] = [b"b", b"a"]
values.sort()
print(values, (b"k", 1), bytes(b"same"))
print(bytes([256]))
