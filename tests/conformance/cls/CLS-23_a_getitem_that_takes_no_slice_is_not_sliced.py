# typy-test: CLS-23
# typy-expect: rejected E0404 at 6:7
class G:
    def __getitem__(self, key: int) -> int:
        return key
print(G()[1:2])
