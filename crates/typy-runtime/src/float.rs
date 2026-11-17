//! `float` operations (DAT-02): arithmetic with Python's rules, exact conversions to and from
//! `int`, `repr`, and `float()` of a string.

mod arithmetic;
pub(crate) mod convert;
mod parse;
mod ratio;
pub(crate) mod repr;
mod round;

pub(crate) use convert::{integer_to_float, typy_rt_float_to_int, typy_rt_int_to_float};
pub(crate) use repr::{shortest, typy_rt_float_to_str};
pub(crate) use round::{power_of_ten, round_quotient};
