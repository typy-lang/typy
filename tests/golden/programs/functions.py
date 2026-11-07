def gcd(a: int, b: int = 12) -> int:
    while b != 0:
        remainder = a % b
        a = b
        b = remainder
    return a


def describe(n: int) -> str:
    if n < 0:
        return "negative"
    elif n == 0:
        return "zero"
    return "positive " + str(n)


for i in range(-1, 3):
    print(describe(i), gcd(i * 18), sep=": ")
print(2 ** 70 // 3, 7 / 2)
