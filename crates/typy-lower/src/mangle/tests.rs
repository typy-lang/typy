//! Tests of `mangle`.

use super::symbol;

#[test]
fn symbols_carry_the_length_of_each_part() {
    assert_eq!(symbol("__main__", "fib"), "_TY8__main__3fib");
    assert_eq!(symbol("app.models", "load"), "_TY10app.models4load");
}
