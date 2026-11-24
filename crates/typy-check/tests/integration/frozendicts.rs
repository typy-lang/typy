//! `frozendict` from the Python floor 3.15, a class of the `typy-stdlib` module `_frozendict`
//! (DAT-18, ADR-0123).

use typy_source::PythonFloor;

use crate::common::messages_at;

#[test]
fn a_frozendict_is_made_from_what_dict_takes_and_read_as_a_dict() {
    let source = "\
prices: frozendict[str, int] = frozendict({'tea': 3})
empty: frozendict[str, int] = frozendict()
pairs = frozendict([('a', 1)])
named = frozendict(a=1, b=2)
again = frozendict(dict(prices))
merged: dict[str, int] = {'x': 1}
merged.update(prices)
merged |= named
print(dict(prices), {**prices}, merged | pairs, list(reversed(prices)), again)
print(prices.get('z', 0) + 1, prices.keys(), prices.copy(), prices | named, hash(prices))
";
    assert_eq!(messages_at(source, PythonFloor::V3_15), Vec::new());
}

#[test]
fn a_frozendict_never_changes() {
    for source in [
        "prices: frozendict[str, int] = frozendict({'tea': 3})\nprices['cake'] = 5\n",
        "prices: frozendict[str, int] = frozendict({'tea': 3})\nprices.pop('tea')\n",
    ] {
        let found = messages_at(source, PythonFloor::V3_15);
        assert_eq!(found.len(), 1, "{source}: {found:?}");
    }
}

#[test]
fn a_module_that_binds_frozendict_keeps_its_own() {
    let source = "\
def frozendict(value: int) -> int:
    return value + 1


print(frozendict(1))
";
    assert_eq!(messages_at(source, PythonFloor::V3_15), Vec::new());
}

#[test]
fn frozendict_below_the_floor_3_15_names_the_floor() {
    for source in [
        "prices = frozendict({'tea': 3})\n",
        "def f(prices: frozendict[str, int]) -> None:\n    pass\n",
        "prices: frozendict = 1\n",
    ] {
        let found = messages_at(source, PythonFloor::V3_14);
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        assert_eq!(found[0].0, "E0425", "{source}");
        assert_eq!(
            found[0].1, "`frozendict` is a builtin from Python 3.15",
            "{source}"
        );
        assert_eq!(found[0].2, "DAT-18", "{source}");
    }
}
