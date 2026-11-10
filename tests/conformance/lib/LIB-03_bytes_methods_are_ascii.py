# typy-test: LIB-03
# typy-expect: accepted
data = b"  Hello, World, hello \x85\xa0 "
print(data.find(b"lo"), data.rfind(b"lo", 0, -3), data.count(b"l"), data.index(b"World"), data.startswith((b"x", b"  H")))
print(data.split(), data.split(b","), data.rsplit(None, 1), b"a\nb\r\nc\x0bd".splitlines(), b"a\nb".splitlines(keepends=True))
print(b"k=v=w".partition(b"="), b"k=v".rpartition(b"x"), b"-".join([b"a", b"b"]), b"".join((b"x", b"y")))
print(data.strip(), data.lstrip(b" H"), b"xxaxx".strip(b"x"), b"TestCase".removeprefix(b"Test"), b"aaa".replace(b"a", b"b", 2))
print(b"ab".center(6, b"*"), b"ab".ljust(4), b"-4".zfill(4), b"a\tb".expandtabs(4), b"\xe9".isascii())
print(b"aBc\xe9 dEF".upper(), b"aBc\xc9".lower(), b"aBc".swapcase(), b"hello world".capitalize(), b"they're bill's".title())
print(b"abc".isalpha(), b"a1".isalnum(), b"12".isdigit(), b" \t".isspace(), b"AB1".isupper(), b"ab".islower(), b"Ab Cd".istitle())
print(b"\x00\xffAB".hex(), b"".hex())
print(b"abc".index(b"z"))
