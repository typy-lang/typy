//! Reads the `[libraries]` table of `typy.toml`: how each C library a `ctypes` declaration
//! names is linked (FFI-04, ADR-0047).

use std::collections::BTreeMap;

use toml_edit::Item;
use typy_diagnostics::RuleId;

use super::{Problem, shown};
use crate::settings::LibraryLink;

fn problem(range: std::ops::Range<usize>, message: String, help: Option<String>) -> Problem {
    Problem {
        range,
        message,
        help,
        rule: RuleId::FFI_04,
    }
}

/// Returns how each library the table names is linked, or what is wrong with it.
pub(super) fn libraries(item: &Item) -> Result<BTreeMap<String, LibraryLink>, Problem> {
    let Some(table) = item.as_table_like() else {
        return Err(problem(
            item.span().unwrap_or(0..0),
            format!("`libraries` is {}, not a table", shown(item)),
            Some(
                "write a table for each library, such as `[libraries.\"libtools.so.3\"]` and \
                 `link = \"static\"`"
                    .to_owned(),
            ),
        ));
    };
    let mut libraries = BTreeMap::new();
    for (name, value) in table.iter() {
        let range = value.span().unwrap_or(0..0);
        if !library_name(name) {
            return Err(problem(
                table
                    .get_key_value(name)
                    .and_then(|(key, _)| key.span())
                    .unwrap_or(range),
                format!("`{name}` is not the file name of a library"),
                Some(
                    "name the library as its `ctypes` declaration does, such as `libm.so.6`"
                        .to_owned(),
                ),
            ));
        }
        let link = value
            .as_table_like()
            .and_then(|entry| {
                (entry.len() == 1)
                    .then(|| entry.get("link"))
                    .flatten()
                    .and_then(Item::as_str)
            })
            .and_then(|link| match link {
                "static" => Some(LibraryLink::Static),
                "dynamic" => Some(LibraryLink::Dynamic),
                _ => None,
            });
        let Some(link) = link else {
            return Err(problem(
                range,
                format!(
                    "`libraries.\"{name}\"` is {}, not a table of one `link`, \"static\" or \
                     \"dynamic\"",
                    shown(value)
                ),
                Some("write `link = \"static\"` or `link = \"dynamic\"`".to_owned()),
            ));
        };
        libraries.insert(name.to_owned(), link);
    }
    Ok(libraries)
}

/// Returns whether `name` is a library's file name, which the linker reads after `-l:` and
/// never as a path or an option.
fn library_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|each| each.is_ascii_alphanumeric() || "_.+-".contains(each))
}

#[cfg(test)]
mod tests;
