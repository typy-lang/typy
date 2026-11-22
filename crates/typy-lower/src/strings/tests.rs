//! Tests of `strings`.

use super::StringPool;

#[test]
fn each_distinct_string_is_kept_once_in_order() {
    let mut pool = StringPool::default();
    let first = pool.intern("a");
    let second = pool.intern("b");
    assert_eq!(pool.intern("a"), first);
    assert_ne!(first, second);
    let surrogate = pool.intern_points(&[0x61, 0xd800]);
    assert_eq!(pool.intern_points(&[0x61, 0xd800]), surrogate);
    assert_eq!(pool.intern_points(&[0xdc00]), surrogate + 1);
    let (strings, points) = pool.into_tables();
    assert_eq!(strings, ["a", "b"]);
    assert_eq!(points, [vec![0x61, 0xd800], vec![0xdc00]]);
}
