# typy-test: DAT-09, MEM-01
# typy-expect: accepted
ages = {"ann": 31, "bob": 42}
for name, age in ages.items():
    print(name, age)
print(ages["cid"])
