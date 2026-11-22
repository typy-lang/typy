//! Tests of `library_kind`.

use super::LibraryKind;

#[test]
fn each_kind_parses_from_its_name() {
    for kind in LibraryKind::ALL {
        assert_eq!(LibraryKind::parse(kind.name()), Some(kind));
    }
    assert_eq!(LibraryKind::parse("dynamic"), None);
}
