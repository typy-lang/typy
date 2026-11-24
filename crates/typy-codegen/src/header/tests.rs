//! Tests of `header`.

use super::c_header;
use typy_tyir::{CType, FunctionId, TyirExport, TyirLibrary};

fn export(name: &str, parameters: &[(&str, CType)], result: Option<CType>) -> TyirExport {
    TyirExport {
        name: name.to_owned(),
        function: FunctionId(0),
        parameters: parameters.iter().map(|(_, ty)| *ty).collect(),
        parameter_names: parameters
            .iter()
            .map(|(name, _)| (*name).to_owned())
            .collect(),
        result,
    }
}

#[test]
fn the_header_declares_each_export_for_c_and_cpp() {
    let library = TyirLibrary {
        name: "geometry".to_owned(),
        version: None,
        exports: vec![
            export(
                "area",
                &[("width", CType::Double), ("height", CType::Double)],
                Some(CType::Double),
            ),
            export("reset", &[], None),
            export(
                "fits",
                &[
                    ("default", CType::Int64),
                    ("new", CType::Bool),
                    ("caf\u{e9}", CType::Int64),
                ],
                Some(CType::Bool),
            ),
        ],
    };
    let text = c_header(&library);
    assert!(text.starts_with("/* geometry.h: the functions the library geometry exports"));
    for line in [
        "#ifndef GEOMETRY_H",
        "#include <stdbool.h>",
        "#include <stdint.h>",
        "extern \"C\" {",
        "double area(double width, double height);",
        "void reset(void);",
        "bool fits(int64_t default_, bool new_, int64_t argument2);",
    ] {
        assert!(text.lines().any(|each| each == line), "{line}\n{text}");
    }
    assert!(text.ends_with("#endif\n"));
    assert!(text.is_ascii());
}
