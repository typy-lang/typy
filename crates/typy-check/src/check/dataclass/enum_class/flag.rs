//! The methods of a `Flag` or an `IntFlag` (CLS-44, ADR-0118), written as CPython 3.12 to 3.15
//! give them to a class whose members are single bits: `|`, `&`, `^` and `~` give the member
//! or the composite of the bits they hold, each composite made once and kept by its value in a
//! hidden module variable, so a second `|` of the same bits gives the same object; a composite
//! is named by its members in definition order, and a value outside the members' bits raises
//! CPython's `ValueError` for a `Flag` and keeps the bits for an `IntFlag`.

use std::fmt::Write as _;

use typy_source::syntax::{Expr, Stmt};

use super::Member;
use crate::class_type::{
    ENUM_BITS, ENUM_FLAG, ENUM_FLAG_BINARY, ENUM_FLAG_WIDTH, ENUM_FLAGS, ENUM_LOOKUP, ENUM_MEMBER,
    ENUM_VALUE,
};

/// The bits a flag class's members use, as CPython's `Flag` reads them.
struct Layout {
    /// The members' bits together, `_flag_mask_`.
    mask: u128,
    /// Every bit up to the highest one used, `_all_bits_`.
    all_bits: u128,
    /// `_all_bits_ + 1`, written in decimal, since it may need 129 bits.
    limit: String,
}

impl Layout {
    fn of(members: &[&Member]) -> Self {
        let mask = members
            .iter()
            .map(|member| bit(member))
            .fold(0, |all, each| all | each);
        let all_bits = u128::MAX.checked_shr(mask.leading_zeros()).unwrap_or(0);
        let limit = all_bits.checked_add(1).map_or_else(
            || "340282366920938463463374607431768211456".to_owned(),
            |limit| limit.to_string(),
        );
        Self {
            mask,
            all_bits,
            limit,
        }
    }
}

/// Returns the source of the methods of a flag class `class` with `members`, single bits, for
/// an `IntFlag` when `int_flag`; the expansion parses it and renames the hidden names.
pub(super) fn source(class: &str, members: &[&Member], int_flag: bool) -> String {
    let bits = Layout::of(members);
    let mut source = format!(
        "def __init__(self, _name_: str | None, _value_: int) -> None:\n    \
         self._name_ = _name_\n    self._value_ = _value_\n\
         @property\ndef name(self) -> str | None:\n    return self._name_\n\
         @property\ndef value(self) -> int:\n    return self._value_\n\
         def __repr__(self) -> str:\n    name = self._name_\n    if name is None:\n        \
         return '<{class}: ' + repr(self._value_) + '>'\n    \
         return '<{class}.' + name + ': ' + repr(self._value_) + '>'\n\
         def __bool__(self) -> bool:\n    return self._value_ != 0\n\
         def __len__(self) -> int:\n    return bin(self._value_).count('1')\n\
         def __contains__(self, other: '{class}') -> bool:\n    \
         return other._value_ & self._value_ == other._value_\n"
    );
    operators(&mut source, class, int_flag, bits.mask);
    let _ = writeln!(
        source,
        "def _bits_(self) -> 'list[{class}]':\n    found: list[{class}] = []"
    );
    for member in members {
        let (value, name) = (bit(member), &member.name.name);
        let _ = writeln!(
            source,
            "    if self._value_ & {value}:\n        found.append({class}.{name})"
        );
    }
    if members.is_empty() {
        let _ = writeln!(
            source,
            "    return found\n\
             @staticmethod\ndef _lookup_(value: int) -> '{class}':\n    \
             raise TypeError(\"<flag '{class}'> has no members; specify `names=()` if you meant \
             to create a new, empty, enum\")"
        );
    } else {
        let _ = writeln!(
            source,
            "    return found\n\
             @staticmethod\ndef _lookup_(value: int) -> '{class}':\n    return {class}._flag_(value)"
        );
    }
    composite(&mut source, class, members, (&bits, int_flag));
    source
}

/// Writes `|`, `&`, `^` and `~`, with their reflected forms for an `IntFlag`, and `__str__`.
fn operators(source: &mut String, class: &str, int_flag: bool, mask: u128) {
    let other = if int_flag {
        "int".to_owned()
    } else {
        format!("'{class}'")
    };
    let other_value = if int_flag { "other" } else { "other._value_" };
    for (method, operator) in [("or", '|'), ("and", '&'), ("xor", '^')] {
        let _ = writeln!(
            source,
            "def __{method}__(self, other: {other}) -> '{class}':\n    \
             return {class}._flag_(self._value_ {operator} {other_value})"
        );
        if int_flag {
            let _ = writeln!(
                source,
                "def __r{method}__(self, other: int) -> '{class}':\n    \
                 return {class}._flag_(other {operator} self._value_)"
            );
        }
    }
    if int_flag {
        let _ = writeln!(
            source,
            "def __str__(self) -> str:\n    return str(self._value_)\n\
             def __invert__(self) -> '{class}':\n    return {class}._flag_(~self._value_)\n\
             def __hash__(self) -> int:\n    return hash(self._value_)\n\
             def _value_of_(self) -> int:\n    return self._value_"
        );
    } else {
        let _ = writeln!(
            source,
            "def __str__(self) -> str:\n    name = self._name_\n    if name is None:\n        \
             return '{class}(' + repr(self._value_) + ')'\n    return '{class}.' + name\n\
             def __invert__(self) -> '{class}':\n    return {class}._flag_({mask} & ~self._value_)"
        );
    }
}

/// Writes the hidden static method that returns the member or composite of a value, as
/// CPython's `Flag._missing_` gives it, with the helpers its `ValueError` uses.
fn composite(
    source: &mut String,
    class: &str,
    members: &[&Member],
    (bits, int_flag): (&Layout, bool),
) {
    let Layout {
        mask,
        all_bits,
        limit,
    } = bits;
    let gaps = all_bits ^ mask;
    let _ = writeln!(
        source,
        "@staticmethod\ndef _flag_(value: int) -> '{class}':\n    \
         known = FLAGS_.get(value)\n    if known is not None:\n        return known\n    \
         given = value\n    \
         if value < -{limit} or value > {all_bits} or value & {gaps} != 0:"
    );
    if int_flag {
        let _ = writeln!(
            source,
            "        if value < 0:\n            \
             value = max({limit}, 1 << {class}._width_(value)) + value"
        );
    } else {
        let _ = writeln!(
            source,
            "        width = max({class}._width_(value), {class}._width_({mask}))\n        \
             raise ValueError(\"<flag '{class}'> invalid value \" + repr(value) + \
             '\\n    given ' + {class}._binary_(value, width) + \
             '\\n  allowed ' + {class}._binary_({mask}, width))"
        );
    }
    let _ = writeln!(source, "    if value < 0:\n        value = {limit} + value");
    for member in members {
        let (value, name) = (bit(member), &member.name.name);
        let _ = writeln!(
            source,
            "    if value == {value}:\n        return {class}.{name}"
        );
    }
    source.push_str("    names: list[str] = []\n");
    for member in members {
        let (value, name) = (bit(member), &member.name.name);
        let _ = writeln!(
            source,
            "    if value & {value}:\n        names.append('{name}')"
        );
    }
    let _ = writeln!(
        source,
        "    name: str | None = None\n    if value & {mask} != 0:\n        \
         name = '|'.join(names)\n        if value & ~{mask} != 0:\n            \
         name = name + '|' + str(value & ~{mask})\n    \
         made = MEMBER_(name, value)\n    FLAGS_[value] = made\n    FLAGS_[given] = made\n    \
         return made\n\
         @staticmethod\ndef _width_(value: int) -> int:\n    if value < 0:\n        \
         return len(bin(value)) - 3\n    if value == 0:\n        return 0\n    \
         return len(bin(value)) - 2\n\
         @staticmethod\ndef _binary_(number: int, width: int) -> str:\n    \
         ceiling = 1 << {class}._width_(number)\n    if number >= 0:\n        \
         text = bin(number + ceiling).replace('1', '0', 1)\n    else:\n        \
         text = bin(~number ^ (ceiling - 1) + ceiling)\n    \
         sign = text[:3]\n    digits = text[3:]\n    if len(digits) < width:\n        \
         digits = (sign[-1] * width + digits)[-width:]\n    return sign + ' ' + digits"
    );
}

/// Returns the bit a member's value is, which the expansion checked is one.
fn bit(member: &Member) -> u128 {
    match &member.value {
        crate::literal_type::LiteralValue::Int(digits) => digits.parse().unwrap_or(0),
        _ => 0,
    }
}

/// Renames the names the source of `source` writes in place of hidden ones, in the methods of
/// the flag class `class`: the class's hidden methods, the module variable of its composites
/// and the call that makes one.
pub(super) fn rename_hidden(methods: &mut [Stmt], class: &str) {
    let renames = [
        ("_flag_", ENUM_FLAG.to_owned()),
        ("_bits_", ENUM_BITS.to_owned()),
        ("_width_", ENUM_FLAG_WIDTH.to_owned()),
        ("_binary_", ENUM_FLAG_BINARY.to_owned()),
        ("_lookup_", ENUM_LOOKUP.to_owned()),
        ("_value_of_", ENUM_VALUE.to_owned()),
        ("FLAGS_", format!("{ENUM_FLAGS}{class}")),
        ("MEMBER_", format!("{ENUM_MEMBER}{class}")),
    ];
    for method in methods {
        if let Stmt::FunctionDef { name, body, .. } = method {
            if let Some((_, hidden)) = renames.iter().find(|(from, _)| *from == name.name) {
                hidden.clone_into(&mut name.name);
            }
            for statement in body {
                rename_statement(statement, &renames);
            }
        }
    }
}

/// Returns the module variable that keeps the composites of the flag class `class`, by value,
/// empty when the class statement runs.
pub(super) fn composites(class: &str, at: typy_diagnostics::Span) -> Stmt {
    let name = |id: &str| Expr::Name {
        id: id.to_owned(),
        span: at,
    };
    Stmt::AnnAssign {
        target: name(&format!("{ENUM_FLAGS}{class}")),
        annotation: Expr::Subscript {
            value: Box::new(name("dict")),
            index: Box::new(Expr::Tuple {
                // The class is defined above the variable, so it is named as it is.
                elements: vec![name("int"), name(class)],
                span: at,
            }),
            span: at,
        },
        value: Some(Expr::Dict {
            keys: Vec::new(),
            values: Vec::new(),
            span: at,
        }),
        simple: true,
        span: at,
    }
}

fn rename_statement(statement: &mut Stmt, renames: &[(&str, String)]) {
    match statement {
        Stmt::Return {
            value: Some(value), ..
        }
        | Stmt::Expr { value, .. } => rename_expression(value, renames),
        Stmt::Assign { targets, value, .. } => {
            for target in targets {
                rename_expression(target, renames);
            }
            rename_expression(value, renames);
        }
        Stmt::AnnAssign { target, value, .. } => {
            rename_expression(target, renames);
            if let Some(value) = value {
                rename_expression(value, renames);
            }
        }
        Stmt::If {
            test, body, orelse, ..
        } => {
            rename_expression(test, renames);
            for each in body.iter_mut().chain(orelse) {
                rename_statement(each, renames);
            }
        }
        Stmt::Raise {
            exception: Some(exception),
            ..
        } => rename_expression(exception, renames),
        _ => {}
    }
}

fn rename_expression(expression: &mut Expr, renames: &[(&str, String)]) {
    match expression {
        Expr::Name { id, .. } => {
            if let Some((_, hidden)) = renames.iter().find(|(from, _)| from == id) {
                hidden.clone_into(id);
            }
        }
        Expr::Attribute {
            value, attribute, ..
        } => {
            rename_expression(value, renames);
            if let Some((_, hidden)) = renames.iter().find(|(from, _)| *from == attribute.name) {
                hidden.clone_into(&mut attribute.name);
            }
        }
        Expr::Call {
            function,
            arguments,
            ..
        } => {
            rename_expression(function, renames);
            for argument in arguments {
                rename_expression(argument, renames);
            }
        }
        Expr::Subscript { value, index, .. } => {
            rename_expression(value, renames);
            rename_expression(index, renames);
        }
        Expr::BinOp { left, right, .. } => {
            rename_expression(left, renames);
            rename_expression(right, renames);
        }
        Expr::UnaryOp { operand, .. } => rename_expression(operand, renames),
        Expr::Compare {
            left, comparators, ..
        } => {
            rename_expression(left, renames);
            for comparator in comparators {
                rename_expression(comparator, renames);
            }
        }
        Expr::BoolOp { values, .. } => {
            for value in values {
                rename_expression(value, renames);
            }
        }
        _ => {}
    }
}
