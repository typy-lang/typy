# typy-test: LIB-03
# typy-expect: accepted
s = "  Hello, World, hello  "
print(s.find("lo"), s.rfind("lo"), s.find("lo", 6), s.find("x"), s.count("l"), s.count("l", 0, 8))
print(s.index("World"), s.rindex("l", 0, -3), s.startswith("  H"), s.endswith(("x", "  ")))
print(s.startswith("Hello", 2), s.endswith("hello", 0, -2), s.startswith(("a", "b")))
print("abc".find(""), "abc".count(""), "abc".find("", 5), "abc".startswith("", 5))
print("abc".index("z"))
