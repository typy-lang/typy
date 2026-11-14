# typy-test: MOD-11
# typy-expect: accepted
# typy-python: 3.15
lazy import settings

print("main starts")
settings.debug = True
settings.count += 2
print(settings.debug, settings.count)


def bump() -> None:
    settings.count += 1


bump()
print(settings.count)
