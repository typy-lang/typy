# typy-test: CST-06
# typy-expect: rejected E0103 at 5:10
word = "a"
match word:
    case f"{word}":
        print("same")
