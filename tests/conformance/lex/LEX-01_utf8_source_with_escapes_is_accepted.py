# typy-test: LEX-01
# typy-expect: accepted
text = "caf\u00e9 \u03b1\U0001f600"
print(text, len(text))
