# typy-test: LIB-03
# typy-expect: accepted
s = "  Hello, World, hello  "
print(s.split(), s.split(","), s.split(", ", 1), s.rsplit(None, 1), s.split(sep="l", maxsplit=2))
print("a\nb\r\nc".splitlines(), "a\nb\r\nc\n".splitlines(keepends=True))
print("k=v=w".partition("="), "k=v=w".rpartition("="), "abc".partition("x"), "abc".rpartition("x"))
print("-".join(["a", "b", "c"]), ", ".join(("x", "y")), "".join("abc"), "+".join(reversed("ab")))
print(" ".join(sorted("cba")), "a,b".split(",")[1], len("x y z".split()))
print("a".split(""))
