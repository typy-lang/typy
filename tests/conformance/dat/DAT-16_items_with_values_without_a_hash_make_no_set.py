# typy-test: DAT-16, DAT-10
# typy-expect: rejected E0413 at 4:7
lists = {"a": [1]}
print(lists.items() | lists.items())
