class Counter:
    def __init__(self) -> None:
        self.count = 0

    def bump(self) -> int:
        from first import VALUE
        self.count += VALUE
        return self.count


def later() -> int:
    import first
    from first import VALUE
    return first.VALUE + VALUE + Counter().bump()
