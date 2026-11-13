from ..helper import HELP
from . import sibling


def describe() -> str:
    return HELP + "/" + sibling.NAME
