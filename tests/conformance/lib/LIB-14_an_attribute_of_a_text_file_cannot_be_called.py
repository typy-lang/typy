# typy-test: LIB-14
# typy-expect: rejected E0401 at 4:6
file = open("x.txt", "w")
file.name()
