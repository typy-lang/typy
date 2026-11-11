import errors

errors.tries += 1
if errors.tries < 2:
    raise ValueError("first try fails")
VALUE = "fragile ok"
