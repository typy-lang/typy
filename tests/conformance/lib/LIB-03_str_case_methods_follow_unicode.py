# typy-test: LIB-03
# typy-expect: accepted
words = ["hello World", "stra\u00dfe", "\u039f\u0394\u03a5\u03a3\u03a3\u0395\u03a5\u03a3", "they're bill's friends",
         "\u01c6emal", "\u0130stanbul", "ABC def"]
for s in words:
    print(s.upper(), s.lower(), s.casefold(), s.swapcase(), s.capitalize(), s.title())
