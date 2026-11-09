# typy-test: LIB-03
# typy-expect: accepted
samples = ["", "abc", "Abc", "ABC", "A1", "123", "\u00b2", "\u00bd", " \t", "_id9", "9id", "Hello World",
           "\u01c5a", "a\x00"]
for s in samples:
    print(repr(s), s.isalpha(), s.isalnum(), s.isdecimal(), s.isdigit(), s.isnumeric(), s.isspace(),
          s.isupper(), s.islower(), s.istitle(), s.isidentifier(), s.isprintable())
print(" \u2003a\u3000".strip(), " a b\u00a0c ".split(), int("\u0661\u0662 "))
