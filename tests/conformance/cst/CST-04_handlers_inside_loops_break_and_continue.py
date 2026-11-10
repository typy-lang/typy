# typy-test: CST-04
# typy-expect: accepted
def risky(n: int) -> int:
    if n > 3:
        raise OverflowError("too big", n)
    return 10 // n


total = 0
for n in [1, 0, 2, 0, 3, 4, 5]:
    try:
        total += risky(n)
    except ZeroDivisionError:
        print("skip zero")
        continue
    except ArithmeticError as error:
        print("stop:", error)
        break
    print("total", total)
print(total)
