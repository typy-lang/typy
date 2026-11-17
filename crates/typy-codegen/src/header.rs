//! Writes the C header of a library: a declaration of each export, with the C types its
//! annotations give, that C and C++ include (FFI-05, ADR-0032).

use std::fmt::Write as _;

use typy_tyir::{TyirLibrary, TyirModule};

/// Names a C or C++ compiler reads as a keyword, or that the header's includes define, which
/// a parameter name cannot be.
const TAKEN: [&str; 64] = [
    "alignas",
    "alignof",
    "auto",
    "bool",
    "case",
    "catch",
    "char",
    "char16_t",
    "char32_t",
    "char8_t",
    "const",
    "consteval",
    "constexpr",
    "constinit",
    "decltype",
    "default",
    "delete",
    "do",
    "double",
    "enum",
    "explicit",
    "export",
    "extern",
    "false",
    "float",
    "friend",
    "goto",
    "inline",
    "int",
    "int64_t",
    "long",
    "mutable",
    "namespace",
    "new",
    "noexcept",
    "nullptr",
    "operator",
    "private",
    "protected",
    "public",
    "register",
    "requires",
    "restrict",
    "short",
    "signed",
    "sizeof",
    "static",
    "static_assert",
    "struct",
    "switch",
    "template",
    "this",
    "thread_local",
    "throw",
    "true",
    "typedef",
    "typeid",
    "typename",
    "union",
    "unsigned",
    "using",
    "virtual",
    "void",
    "volatile",
];

/// Returns the header of the library `module` is built as, or `None` for an executable.
#[must_use]
pub fn c_header(module: &TyirModule) -> Option<String> {
    let library = module.library.as_ref()?;
    Some(header(library))
}

fn header(library: &TyirLibrary) -> String {
    let guard = format!("{}_H", library.name.to_ascii_uppercase());
    let mut text = format!(
        "/* {name}.h: the functions the library {name} exports, built by TyPy {version}. */\n\
         #ifndef {guard}\n#define {guard}\n\n#include <stdbool.h>\n#include <stdint.h>\n\n\
         #ifdef __cplusplus\nextern \"C\" {{\n#endif\n\n",
        name = library.name,
        version = env!("CARGO_PKG_VERSION"),
    );
    for export in &library.exports {
        let parameters: Vec<String> = export
            .parameters
            .iter()
            .zip(&export.parameter_names)
            .enumerate()
            .map(|(index, (ty, name))| format!("{} {}", ty.c_name(), parameter_name(name, index)))
            .collect();
        let parameters = if parameters.is_empty() {
            "void".to_owned()
        } else {
            parameters.join(", ")
        };
        let result = export.result.map_or("void", |result| result.c_name());
        let _ = writeln!(text, "{result} {}({parameters});", export.name);
    }
    text.push_str("\n#ifdef __cplusplus\n}\n#endif\n\n#endif\n");
    text
}

/// Returns a parameter's name in the header: its Python name, with an underscore after one
/// that C or C++ would read as something else, or a name of its position for one that is
/// not ASCII.
fn parameter_name(name: &str, index: usize) -> String {
    if !name.is_ascii() {
        format!("argument{index}")
    } else if TAKEN.contains(&name) {
        format!("{name}_")
    } else {
        name.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::header;
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
        let text = header(&library);
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
}
