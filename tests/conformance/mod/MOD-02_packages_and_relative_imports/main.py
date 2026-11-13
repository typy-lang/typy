# typy-test: MOD-02, MOD-01
# typy-expect: accepted
from pkg.inner import leaf
import pkg
import pkg.inner.sibling as sibling

print(leaf.describe(), pkg.helper.HELP, sibling.NAME, pkg.inner.sibling is sibling)
print(__name__)


def again() -> None:
    import main
    print("main imported as", main.__name__)


if __name__ == "__main__":
    again()
