# typy-test: EXP-07
# typy-expect: accepted
print("ell" in "hello", "" in "", "z" not in "abc", "\U0001f600" in "a\U0001f600")
pair = (1, 2.5)
print(1 in pair, 2.5 in pair, 3 in pair, 3 not in pair, 2 in (1.0, 2.0))
words = ("a", "b")
print("b" in words, "c" in words, [1] in ([1], [2]), (1, "a") in ((1, "a"), (2, "b")))
