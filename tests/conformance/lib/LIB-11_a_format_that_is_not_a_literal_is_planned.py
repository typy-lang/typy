# typy-test: LIB-11
# typy-expect: rejected E0201 at 4:7
template = "%s items"
print(template % 3)
