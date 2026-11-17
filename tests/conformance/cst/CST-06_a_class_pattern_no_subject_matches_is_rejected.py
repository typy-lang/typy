# typy-test: CST-06
# typy-expect: rejected E0407 at 5:10
word = "text"
match word:
    case int(number):
        print(number)
