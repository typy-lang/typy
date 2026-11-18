//! Tests of the operators each pair of operand types takes, and of the special methods that
//! define unary operators, against what CPython runs.

use crate::common::app;

/// Asserts that each program is refused with `code` first.
fn refused_with(code: &str, cases: &[&str]) {
    for source in cases {
        let found = app(source);
        assert_eq!(
            found.first().map(String::as_str),
            Some(code),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn each_bitwise_operator_on_a_bool_and_an_int_is_bool_arithmetic() {
    refused_with(
        "E0406",
        &[
            "print(True & 1)\n",
            "print(1 & True)\n",
            "print(True | 1)\n",
            "print(1 | True)\n",
            "print(True ^ 1)\n",
            "print(1 ^ True)\n",
        ],
    );
}

#[test]
fn set_operators_take_two_sets_of_one_item_type() {
    refused_with(
        "E0401",
        &[
            "print({1} | {'a'})\n",
            "print({1} & {'a'})\n",
            "print({1} - {'a'})\n",
            "print({1} ^ {'a'})\n",
        ],
    );
}

#[test]
fn arithmetic_on_a_number_and_a_value_that_is_not_one_is_refused() {
    refused_with(
        "E0401",
        &[
            "print(1 + 'a')\n",
            "print(1.5 + [1])\n",
            "print(1 - 'a')\n",
            "print(1.5 - [1])\n",
            "print(1.5 * [1])\n",
            "print(1 // 'a')\n",
            "print(1.5 // [1])\n",
            "print(1 % 'a')\n",
            "print(1.5 % [1])\n",
        ],
    );
}

#[test]
fn shifts_and_bitwise_operators_take_two_ints() {
    refused_with(
        "E0401",
        &[
            "print(1 << 1.0)\n",
            "print(1.0 << 1)\n",
            "print(1 >> 1.0)\n",
            "print(1.0 >> 1)\n",
            "print(1 & 1.0)\n",
            "print(1.0 & 1)\n",
            "print(1 | 1.0)\n",
            "print(1.0 | 1)\n",
            "print(1 ^ 1.0)\n",
            "print(1.0 ^ 1)\n",
        ],
    );
}

#[test]
fn a_membership_test_for_a_value_no_item_can_be_is_refused() {
    refused_with(
        "E0407",
        &[
            "print('a' in [1])\n",
            "print('a' in (1, 2))\n",
            "print('a' in {1})\n",
            "print('a' in {1: 2})\n",
            "t: tuple[int, ...] = (1, 2)\nprint('a' in t)\n",
        ],
    );
}

#[test]
fn a_view_of_keys_combines_with_a_list_of_its_items() {
    assert_eq!(
        app("d = {1: 2}\nprint(d.keys() | [1])\n"),
        Vec::<String>::new()
    );
    refused_with("E0401", &["d = {1: 2}\nprint(d.keys() | ['a'])\n"]);
}

#[test]
fn unary_special_methods_take_only_the_instance() {
    for source in [
        "class A:\n    def __neg__(self) -> 'A':\n        return self\nprint(-A())\n",
        "class A:\n    def __pos__(self) -> 'A':\n        return self\nprint(+A())\n",
        "class A:\n    def __invert__(self) -> 'A':\n        return self\nprint(~A())\n",
        "class A:\n    def __abs__(self) -> int:\n        return 1\nprint(abs(A()))\n",
    ] {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
    let refused: Vec<String> = [
        "__neg__",
        "__pos__",
        "__invert__",
        "__abs__",
        "__iter__",
        "__next__",
        "__enter__",
    ]
    .iter()
    .map(|name| {
        format!("class A:\n    def {name}(self, other: 'A') -> 'A':\n        return self\n")
    })
    .chain(["__str__", "__repr__"].iter().map(|name| {
        format!("class A:\n    def {name}(self, other: 'A') -> str:\n        return ''\n")
    }))
    .collect();
    refused_with(
        "E0404",
        &refused.iter().map(String::as_str).collect::<Vec<_>>(),
    );
}
