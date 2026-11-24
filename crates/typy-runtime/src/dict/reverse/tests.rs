//! Tests of stepping backwards through a dict's entries.

use super::{typy_rt_dict_last_entry, typy_rt_dict_previous};
use crate::dict::{typy_rt_dict_new, typy_rt_dict_set};
use crate::list::ItemKind;
use crate::word::{inline, inline_value};

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

#[test]
fn reversed_entries_skip_holes_and_notice_a_change_of_size() {
    let int = word(ItemKind::Int as i64);
    let dict = typy_rt_dict_new(int, 0, int, 0, word(0));
    // SAFETY: the dict is live, and every key and value is an inline word.
    unsafe {
        assert_eq!(inline_value(typy_rt_dict_last_entry(dict)), -1);
        for key in 0..4 {
            typy_rt_dict_set(dict, word(key).cast_unsigned(), word(key).cast_unsigned());
        }
        let popped = crate::dict::typy_rt_dict_pop(dict, word(2).cast_unsigned());
        assert!(popped.exception.is_null());
        let last = typy_rt_dict_last_entry(dict);
        assert_eq!(inline_value(last), 3);
        let length = word(3);
        let mut seen = Vec::new();
        let mut position = last;
        loop {
            let found = typy_rt_dict_previous(dict, position, length);
            assert!(found.exception.is_null());
            let index = inline_value(found.value);
            if index < 0 {
                break;
            }
            seen.push(index);
            position = word(index - 1);
        }
        assert_eq!(seen, [3, 1, 0]);
        assert_eq!(
            inline_value(typy_rt_dict_previous(dict, word(99), length).value),
            3
        );
        let changed = typy_rt_dict_previous(dict, last, word(4));
        // SAFETY: the exception is live and owned here.
        assert_eq!(
            (*changed.exception).kind(),
            crate::ExceptionKind::RuntimeError
        );
        crate::refcount::typy_rt_decrement(changed.exception.cast());
        crate::refcount::typy_rt_decrement(dict.cast());
    }
}
