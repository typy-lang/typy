//! `float` operations (DAT-02): arithmetic with Python's rules, exact conversions to and from
//! `int`, `repr`, and `float()` of a string.

mod arithmetic;
mod convert;
mod parse;
mod ratio;
mod repr;

pub(crate) use repr::typy_rt_float_to_str;
