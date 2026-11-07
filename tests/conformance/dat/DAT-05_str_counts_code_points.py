# typy-test: DAT-05
# typy-expect: accepted
text = "a\u00e9\U0001f600"
print(len(text), text[0], text[1], text[-1], len(text[2]), len(""))
