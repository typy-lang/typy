# typy-test: LIB-03
# typy-expect: accepted
s = "  Hello, World, hello  "
print(repr(s.strip()), repr(s.lstrip()), repr(s.rstrip(" o")), "xxaxx".strip("x"), s.strip(None))
print("TestCase".removeprefix("Test"), "file.py".removesuffix(".py"), "aaa".replace("a", "b", 2))
print("abc".replace("", "-"), "abc".replace("", "-", 2))
print(repr("ab".center(7, "*")), repr("ab".ljust(5)), repr("ab".rjust(5, "0")), "-42".zfill(6))
print(repr("a\tbc\td".expandtabs(4)), repr("a\tb".expandtabs()), "abc".isascii(), "\u00e9".isascii())
print("ab".center(5, "--"))
