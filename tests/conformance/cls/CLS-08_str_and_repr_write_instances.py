# typy-test: CLS-08
# typy-expect: accepted
class Card:
    def __init__(self, rank: str, suit: str) -> None:
        self.rank = rank
        self.suit = suit

    def __repr__(self) -> str:
        return "Card(" + repr(self.rank) + ", " + repr(self.suit) + ")"

    def __str__(self) -> str:
        return self.rank + " of " + self.suit


class Hand:
    def __init__(self) -> None:
        self.cards: list[Card] = []

    def __repr__(self) -> str:
        return "Hand" + repr(self.cards)


hand = Hand()
hand.cards.append(Card("ace", "spades"))
hand.cards.append(Card("2", "hearts"))
print(hand.cards[0], str(hand.cards[1]), repr(hand.cards[0]))
print(hand, hand.cards, (hand.cards[1], 1), {"top": hand.cards[0]})
plain = Hand()
print(str(plain) == repr(plain), [plain])
