# typy-test: DAT-13
# typy-expect: rejected E0402 at 5:1
values = [1, 2, 3]
words = ["a"]
values[0:2] = words
