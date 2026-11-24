//! Tests of `class_type`.

use super::InstanceClass;
use crate::typed::ClassId;

fn chain() -> (InstanceClass, InstanceClass, InstanceClass) {
    let shape = InstanceClass::root(ClassId(0), "Shape");
    let circle = InstanceClass {
        id: ClassId(1),
        name: "Circle".to_owned(),
        ancestors: vec![(ClassId(0), "Shape".to_owned())],
        operators: super::Operators::default(),
        generic: None,
        protocol: false,
    };
    let disc = InstanceClass {
        id: ClassId(2),
        name: "Disc".to_owned(),
        ancestors: vec![
            (ClassId(1), "Circle".to_owned()),
            (ClassId(0), "Shape".to_owned()),
        ],
        operators: super::Operators::default(),
        generic: None,
        protocol: false,
    };
    (shape, circle, disc)
}

#[test]
fn a_class_derives_from_itself_and_each_ancestor_but_not_a_subclass() {
    let (shape, circle, disc) = chain();
    assert!(disc.derives_from(&disc));
    assert!(disc.derives_from(&circle));
    assert!(disc.derives_from(&shape));
    assert!(!shape.derives_from(&circle));
}

#[test]
fn the_base_keeps_the_rest_of_the_chain() {
    let (shape, circle, disc) = chain();
    assert_eq!(disc.base(), Some(circle));
    assert_eq!(shape.base(), None);
}

#[test]
fn the_common_base_is_the_nearest_shared_class() {
    let (shape, circle, disc) = chain();
    let square = InstanceClass {
        id: ClassId(3),
        name: "Square".to_owned(),
        ancestors: vec![(ClassId(0), "Shape".to_owned())],
        operators: super::Operators::default(),
        generic: None,
        protocol: false,
    };
    assert_eq!(disc.common_base(&circle), Some(circle.clone()));
    assert_eq!(disc.common_base(&square), Some(shape));
    assert_eq!(
        square.common_base(&InstanceClass::root(ClassId(4), "Other")),
        None
    );
}

#[test]
fn a_class_with_eq_and_no_hash_has_no_hash_and_lt_orders() {
    use super::Operators;
    let root = Operators::default();
    let offers = |operators: Operators| {
        (
            operators.hashable(),
            operators.orders(),
            operators.compares(),
        )
    };
    assert_eq!(offers(root), (true, false, false));
    let equal = Operators::of(["__eq__"], root);
    assert_eq!(offers(equal), (false, false, true));
    let hashed = Operators::of(["__eq__", "__hash__"], root);
    assert!(hashed.hashable());
    let child = Operators::of(["__lt__"], hashed);
    assert_eq!(offers(child), (true, true, true));
    let redefined = Operators::of(["__eq__"], hashed);
    assert!(!redefined.hashable());
    assert!(Operators::of(["__gt__"], equal).orders());
}
