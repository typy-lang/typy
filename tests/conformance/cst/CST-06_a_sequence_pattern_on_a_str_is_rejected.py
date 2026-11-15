# typy-test: CST-06
# typy-expect: rejected E0407 at 5:10
word = "ab"
match word:
    case [first, *rest]:
        print(first, rest)
