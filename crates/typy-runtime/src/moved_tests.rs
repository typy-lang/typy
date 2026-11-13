//! Tests of the stores that take over the caller's reference to an item (MEM-09): each leaves
//! the item with one reference fewer than the counting store would, and releases an item it
//! does not keep.

use crate::list::{ItemKind, ListObject, new_list};
use crate::refcount::{typy_rt_decrement, typy_rt_increment};
use crate::string::{StrObject, from_str};

/// Returns the reference count of a `str`.
fn count(text: *mut StrObject) -> u32 {
    // SAFETY: the string is live.
    unsafe { (*text).header.reference_count }
}

/// Returns a new `str` with a second reference, which the test keeps to watch the count.
fn watched(text: &str) -> *mut StrObject {
    let object = from_str(text);
    // SAFETY: the string is live.
    unsafe { typy_rt_increment(object.cast()) };
    object
}

fn word(value: i64) -> i64 {
    crate::word::inline(value).unwrap()
}

fn release(objects: &[*mut crate::header::ObjectHeader]) {
    for object in objects {
        // SAFETY: each object is live, and this is a reference the test owns.
        unsafe { typy_rt_decrement(*object) };
    }
}

#[test]
fn a_list_takes_over_an_appended_inserted_or_set_item() {
    let list: *mut ListObject = new_list(ItemKind::Object, 0);
    let (appended, inserted, set, refused) =
        (watched("a"), watched("b"), watched("c"), watched("d"));
    // SAFETY: the list and the items are live; each store takes one of their references.
    unsafe {
        crate::list::typy_rt_list_append_moved(list, appended as u64);
        crate::list::typy_rt_list_insert_moved(list, word(0), inserted as u64);
        assert!(crate::list::typy_rt_list_set_moved(list, word(1), set as u64).is_null());
        // The appended item was replaced, so the list released it.
        assert_eq!((count(appended), count(inserted), count(set)), (1, 2, 2));
        let error = crate::list::typy_rt_list_set_moved(list, word(9), refused as u64);
        assert!(!error.is_null());
        assert_eq!(count(refused), 1);
        typy_rt_decrement(error.cast());
        assert_eq!((*list).length, 2);
    }
    release(&[
        list.cast(),
        appended.cast(),
        inserted.cast(),
        set.cast(),
        refused.cast(),
    ]);
}

#[test]
fn the_counting_stores_leave_the_caller_its_reference() {
    let list: *mut ListObject = new_list(ItemKind::Object, 0);
    let item = watched("kept");
    // SAFETY: the list and the item are live.
    unsafe {
        crate::list::typy_rt_list_append(list, item as u64);
        crate::list::typy_rt_list_insert(list, word(0), item as u64);
        let error = crate::list::typy_rt_list_set(list, word(9), item as u64);
        assert!(!error.is_null());
        typy_rt_decrement(error.cast());
    }
    assert_eq!(count(item), 4);
    release(&[list.cast(), item.cast(), item.cast()]);
}

#[test]
fn a_set_takes_over_a_new_item_and_releases_one_it_has() {
    let set = crate::set::new_set(ItemKind::Object.into());
    let (first, equal) = (watched("same"), watched("same"));
    // SAFETY: the set and the items are live; each add takes one of their references.
    unsafe {
        crate::set::typy_rt_set_add_moved(set, first as u64);
        crate::set::typy_rt_set_add_moved(set, equal as u64);
    }
    assert_eq!((count(first), count(equal)), (2, 1));
    release(&[set.cast(), first.cast(), equal.cast()]);
}

#[test]
fn a_dict_takes_over_a_value_and_counts_its_key() {
    let dict = crate::dict::new_dict(ItemKind::Object.into(), ItemKind::Object.into(), 0);
    let (key, value, replacing) = (watched("key"), watched("value"), watched("replacing"));
    // SAFETY: the dict and the items are live; each set takes a reference to its value.
    unsafe {
        crate::dict::typy_rt_dict_set_moved(dict, key as u64, value as u64);
        assert_eq!((count(key), count(value)), (3, 2));
        crate::dict::typy_rt_dict_set_moved(dict, key as u64, replacing as u64);
    }
    assert_eq!((count(key), count(value), count(replacing)), (3, 1, 2));
    release(&[
        dict.cast(),
        key.cast(),
        key.cast(),
        value.cast(),
        replacing.cast(),
    ]);
}
