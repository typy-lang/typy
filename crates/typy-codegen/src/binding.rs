//! Writes the Python files of a library: a `ctypes` module that loads it and defines a typed
//! function for each export, and a stub of those functions (FFI-08, ADR-0032, ADR-0046).

use std::fmt::Write as _;

use typy_tyir::{CType, TyirExport, TyirLibrary};

/// Returns the stub of the binding module of `library`.
#[must_use]
pub fn python_stub(library: &TyirLibrary) -> String {
    let mut text = format!(
        "\"\"\"The functions of the C library lib{name}, built by TyPy {version} from \
         {name}.py, as {name}_binding calls them.\"\"\"\n\n__all__ = [{all}]\n\n",
        name = library.name,
        version = env!("CARGO_PKG_VERSION"),
        all = listed(library),
    );
    for export in &library.exports {
        let _ = writeln!(text, "def {}: ...", signature(export));
    }
    text
}

/// Returns the binding module of `library`, which loads the shared library `soname` with
/// `ctypes`.
#[must_use]
pub fn python_binding(library: &TyirLibrary, soname: &str) -> String {
    let handle = handle_name(library);
    let mut text = format!(
        "\"\"\"The C library lib{name}, built by TyPy {version} from {name}.py: loads it with \
         ctypes and calls its functions.\"\"\"\n\nimport ctypes\n\n__all__ = [{all}]\n\n\
         {handle} = ctypes.CDLL({soname})\n",
        name = library.name,
        version = env!("CARGO_PKG_VERSION"),
        all = listed(library),
        soname = python_string(soname),
    );
    for export in &library.exports {
        let argument_types: Vec<&str> = export
            .parameters
            .iter()
            .map(|ty| ctypes_name(*ty))
            .collect();
        let result_type = export.result.map_or("None", ctypes_name);
        let _ = writeln!(
            text,
            "{handle}.{name}.argtypes = [{}]\n{handle}.{name}.restype = {result_type}",
            argument_types.join(", "),
            name = export.name,
        );
    }
    for export in &library.exports {
        let call = format!(
            "{handle}.{}({})",
            export.name,
            export.parameter_names.join(", ")
        );
        let body = if export.result.is_some() {
            format!("return {call}")
        } else {
            call
        };
        let _ = write!(text, "\n\ndef {}:\n    {body}\n", signature(export));
    }
    text
}

/// Returns `name(parameter: type, ...) -> type` of an export, with its Python types.
fn signature(export: &TyirExport) -> String {
    let parameters: Vec<String> = export
        .parameter_names
        .iter()
        .zip(&export.parameters)
        .map(|(name, ty)| format!("{name}: {}", python_name(*ty)))
        .collect();
    let result = export.result.map_or("None", python_name);
    format!("{}({}) -> {result}", export.name, parameters.join(", "))
}

/// Returns the names of the exports as the items of `__all__`.
fn listed(library: &TyirLibrary) -> String {
    let names: Vec<String> = library
        .exports
        .iter()
        .map(|export| python_string(&export.name))
        .collect();
    names.join(", ")
}

/// Returns the name of the module variable that holds the library: `_library`, with
/// underscores after it until no export or parameter has its name, which would hide it.
fn handle_name(library: &TyirLibrary) -> String {
    let taken = |name: &str| {
        library.exports.iter().any(|export| {
            export.name == name || export.parameter_names.iter().any(|each| each == name)
        })
    };
    let mut name = "_library".to_owned();
    while taken(&name) {
        name.push('_');
    }
    name
}

const fn python_name(ty: CType) -> &'static str {
    match ty {
        CType::Float | CType::Double => "float",
        CType::Bool => "bool",
        CType::CharPointer => "bytes | None",
        CType::VoidPointer | CType::Pointer(_) | CType::Structure(_) => "int | None",
        _ => "int",
    }
}

const fn ctypes_name(ty: CType) -> &'static str {
    match ty {
        CType::Int8 => "ctypes.c_int8",
        CType::UInt8 => "ctypes.c_uint8",
        CType::Int16 => "ctypes.c_int16",
        CType::UInt16 => "ctypes.c_uint16",
        CType::Int32 => "ctypes.c_int32",
        CType::UInt32 => "ctypes.c_uint32",
        CType::Int64 => "ctypes.c_int64",
        CType::UInt64 => "ctypes.c_uint64",
        CType::Float => "ctypes.c_float",
        CType::Double => "ctypes.c_double",
        CType::Bool => "ctypes.c_bool",
        CType::CharPointer => "ctypes.c_char_p",
        CType::VoidPointer | CType::Pointer(_) | CType::Structure(_) => "ctypes.c_void_p",
    }
}

/// Returns `text` as a Python string literal in double quotes.
fn python_string(text: &str) -> String {
    let mut literal = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            control if control.is_control() => {
                let _ = write!(literal, "\\u{:04x}", u32::from(control));
            }
            other => literal.push(other),
        }
    }
    literal.push('"');
    literal
}

#[cfg(test)]
mod tests;
