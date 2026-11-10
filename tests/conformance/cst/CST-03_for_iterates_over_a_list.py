# typy-test: CST-03
# typy-expect: accepted
words = ["a", "b"]
for word in words:
    print(word)
    if len(words) < 4:
        words.append(word + "!")
for n in [1, 2, 3]:
    if n == 2:
        continue
    print(n)
else:
    print("done")
empty: list[int] = []
for n in empty:
    print("never")
else:
    print("empty loop")
