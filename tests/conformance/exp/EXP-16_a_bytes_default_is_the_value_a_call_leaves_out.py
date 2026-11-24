# typy-test: EXP-16
# typy-expect: accepted
# A `bytes` constant is a default value as a `str` constant is: a call that leaves the
# parameter out passes it.
def joined(parts: list[bytes], separator: bytes = b", ", end: bytes | None = b".") -> bytes:
    text = separator.join(parts)
    if end is not None:
        text += end
    return text


print(joined([b"a", b"b"]), joined([b"a"], b"-"), joined([b"a", b"b"], end=None))
