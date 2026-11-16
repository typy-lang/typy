"""A small interpreter: arithmetic expressions written out, tokenized and evaluated."""

import sys

MODULUS = 1000003


def expression(seed: int) -> str:
    parts: list[str] = []
    for k in range(12):
        value = (seed * 31 + k * 17) % 97 + 1
        if k % 4 == 0:
            parts.append("(" + str(value))
        elif k % 4 == 3:
            parts.append(str(value) + ")")
        else:
            parts.append(str(value))
    operators = ["+", "*", "-", "+"]
    text = parts[0]
    for k in range(1, 12):
        text += " " + operators[(seed + k) % 4] + " " + parts[k]
    return text


def tokens(text: str) -> list[str]:
    found: list[str] = []
    number = ""
    for character in text:
        if character.isdigit():
            number += character
            continue
        if number:
            found.append(number)
            number = ""
        if character != " ":
            found.append(character)
    if number:
        found.append(number)
    return found


class Parser:
    def __init__(self, items: list[str]) -> None:
        self.items = items
        self.position = 0

    def peek(self) -> str:
        if self.position < len(self.items):
            return self.items[self.position]
        return ""

    def take(self) -> str:
        item = self.items[self.position]
        self.position += 1
        return item

    def sum(self) -> int:
        value = self.product()
        while self.peek() == "+" or self.peek() == "-":
            if self.take() == "+":
                value = (value + self.product()) % MODULUS
            else:
                value = (value - self.product()) % MODULUS
        return value

    def product(self) -> int:
        value = self.atom()
        while self.peek() == "*":
            self.take()
            value = value * self.atom() % MODULUS
        return value

    def atom(self) -> int:
        item = self.take()
        if item == "(":
            value = self.sum()
            self.take()
            return value
        return int(item)


def main() -> None:
    total = 0
    for i in range(int(sys.argv[1])):
        total = (total + Parser(tokens(expression(i))).sum()) % MODULUS
    print(total)


main()
