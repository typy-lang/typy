//! Tests of `body::import`.

#[test]
fn each_name_a_refusal_lists_is_one_the_module_gives() {
    for module in ["typing", "collections.abc", "dataclasses"] {
        for name in super::given_names(module) {
            assert!(
                crate::check::annotation_name(module, name).is_some(),
                "{module}.{name}"
            );
        }
    }
}
