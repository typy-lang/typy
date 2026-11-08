# typy-test: DAT-12
# typy-expect: accepted
text = "na\u00efve caf\u00e9"
print(text[2:5], text[::-1], text[-4:], text[::3], text[7:2], repr(text[3:3]))
wide = "a\U0001f600b"
print(len(wide[1:]), wide[1:2] == "\U0001f600", wide[::2])
