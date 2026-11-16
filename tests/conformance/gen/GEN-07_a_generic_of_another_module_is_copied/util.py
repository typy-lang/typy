SCALE = 10


def scaled[T: (int, float)](value: T) -> T:
    return value * SCALE
