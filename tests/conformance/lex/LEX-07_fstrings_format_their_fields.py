# typy-test: LEX-07
# typy-expect: accepted
class Item:
    def __init__(self, name: str, price: float, count: int) -> None:
        self.name = name
        self.price = price
        self.count = count

    def __repr__(self) -> str:
        return f"Item({self.name!r}, {self.price}, {self.count})"


items = [Item("tea", 3.5, 12), Item("café", 12.25, 3), Item("cake", 1234.5, 1500)]
width = 8
for item in items:
    total = item.price * item.count
    print(f"{item.name:<{width}}|{item.count:>6,}|{item.price:8.2f}|{total:>12,.2f}|")
print(f"{items[0]!r} {items[1].name!a} {len(items) = } {0.125:.1%} {255:#06x} {255:_b}")
print(f"{12345.678:e} {12345.678:.3g} {1e-7:g} {-0.0:z.2f} {True:>5} {True:d} {None}")
best: Item | None = None
print(f"{best} {best!r:>6}")
best = items[2]
print(f"{best.count if best is not None else 0:+d} {'a' 'b'} {{braces}} {f'{width:03}'}")
