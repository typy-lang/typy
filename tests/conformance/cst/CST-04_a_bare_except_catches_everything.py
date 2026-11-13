# typy-test: CST-04
# typy-expect: accepted
count = 0
for text in ["1", "x", "3"]:
    try:
        count += int(text)
    except:
        print("not a number:", text)
print(count)
