# typy-test: CST-04
# typy-expect: accepted
def divide(a: int, b: int) -> str:
    try:
        return str(a // b)
    except ZeroDivisionError:
        return "inf"
    finally:
        print("divided", a, b)


def label(n: int) -> str:
    text = "n=" + str(n)
    try:
        if n > 1:
            return text * 2
    finally:
        print("finally for", n)
    return text


print(divide(7, 2), divide(1, 0))
print(label(1), label(2))
for n in range(4):
    try:
        if n == 1:
            continue
        if n == 3:
            break
        print("body", n)
    finally:
        print("cleanup", n)
try:
    try:
        raise KeyError("inner")
    finally:
        print("inner finally")
except LookupError as error:
    print("outer caught", repr(error))
finally:
    print("outer finally")
count = 0
try:
    count = 1
finally:
    count += 10
print(count)
