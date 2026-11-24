//! Tests of `exception`.

use super::{BuiltinException, ExceptionClass};

#[test]
fn every_class_is_found_by_its_name_and_reaches_base_exception() {
    for class in BuiltinException::ALL {
        assert_eq!(BuiltinException::named(class.name()), Some(class));
        assert!(
            ExceptionClass::Builtin(class)
                .derives_from(&ExceptionClass::Builtin(BuiltinException::BaseException)),
            "{class:?}"
        );
    }
    assert_eq!(BuiltinException::named("Warning"), None);
}

#[test]
fn a_class_derives_from_its_bases_only() {
    let derives =
        |class, base| ExceptionClass::Builtin(class).derives_from(&ExceptionClass::Builtin(base));
    assert!(derives(
        BuiltinException::KeyError,
        BuiltinException::LookupError
    ));
    assert!(derives(
        BuiltinException::UnicodeDecodeError,
        BuiltinException::ValueError
    ));
    assert!(derives(
        BuiltinException::NotImplementedError,
        BuiltinException::RuntimeError
    ));
    assert!(!derives(
        BuiltinException::KeyError,
        BuiltinException::IndexError
    ));
    assert!(!derives(
        BuiltinException::Exception,
        BuiltinException::ValueError
    ));
}

#[test]
fn the_common_base_is_the_nearest_class_all_derive_from() {
    let common = |classes: &[BuiltinException]| {
        let classes: Vec<_> = classes
            .iter()
            .copied()
            .map(ExceptionClass::Builtin)
            .collect();
        super::common_base(&classes)
    };
    let cases: [(&[BuiltinException], BuiltinException); 5] = [
        (
            &[BuiltinException::KeyError, BuiltinException::IndexError],
            BuiltinException::LookupError,
        ),
        (
            &[
                BuiltinException::KeyError,
                BuiltinException::ZeroDivisionError,
            ],
            BuiltinException::Exception,
        ),
        (
            &[BuiltinException::ValueError],
            BuiltinException::ValueError,
        ),
        (
            &[BuiltinException::UnicodeError, BuiltinException::ValueError],
            BuiltinException::ValueError,
        ),
        (&[], BuiltinException::BaseException),
    ];
    for (classes, expected) in cases {
        assert_eq!(
            common(classes),
            ExceptionClass::Builtin(expected),
            "{classes:?}"
        );
    }
}

/// Returns a class of the program derived from `ancestors`, nearest first, then `builtin`.
fn program_class(id: u32, ancestors: &[u32], builtin: BuiltinException) -> ExceptionClass {
    ExceptionClass::program(
        crate::typed::ClassId(id),
        format!("C{id}"),
        ancestors
            .iter()
            .map(|each| (crate::typed::ClassId(*each), format!("C{each}")))
            .collect(),
        builtin,
    )
}

#[test]
fn a_program_class_derives_from_its_ancestors_and_their_built_in_bases() {
    let app = program_class(0, &[], BuiltinException::ValueError);
    let missing = program_class(1, &[0], BuiltinException::ValueError);
    let other = program_class(2, &[], BuiltinException::KeyError);
    assert!(missing.derives_from(&app));
    assert!(missing.derives_from(&missing));
    assert!(missing.derives_from(&ExceptionClass::Builtin(BuiltinException::Exception)));
    assert!(!app.derives_from(&missing));
    assert!(!other.derives_from(&app));
    assert!(!ExceptionClass::Builtin(BuiltinException::ValueError).derives_from(&app));
    assert_eq!(missing.base(), Some(app.clone()));
    assert_eq!(
        app.base(),
        Some(ExceptionClass::Builtin(BuiltinException::ValueError))
    );
    assert_eq!(super::common_base(&[missing.clone(), app.clone()]), app);
    assert_eq!(
        super::common_base(&[missing, other]),
        ExceptionClass::Builtin(BuiltinException::Exception)
    );
}

#[test]
fn only_the_unicode_errors_cannot_be_called() {
    let refused: Vec<_> = BuiltinException::ALL
        .into_iter()
        .filter(|class| !class.can_be_called())
        .collect();
    assert_eq!(
        refused,
        [
            BuiltinException::UnicodeDecodeError,
            BuiltinException::UnicodeEncodeError
        ]
    );
}
