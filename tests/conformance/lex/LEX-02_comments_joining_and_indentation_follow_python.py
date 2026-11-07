# typy-test: LEX-02
# typy-expect: accepted
# a comment line
total = 1 + \
    2  # an explicit join
value = (3 +
         4)  # an implicit join
if total < value:
	print(total, value)
while False:
        pass
