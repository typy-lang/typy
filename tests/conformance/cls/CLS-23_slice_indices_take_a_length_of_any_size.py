# typy-test: CLS-23
# typy-expect: accepted
part = slice(1, 3)
print(part.indices(10**30))
print(slice(None, None, -1).indices(2**70))
print(slice(-5, None).indices(10**20))
print(slice(10**25, -(10**25), -2).indices(10**24))
