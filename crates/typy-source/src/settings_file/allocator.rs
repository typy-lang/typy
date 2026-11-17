//! Reads the `[allocator]` table of `typy.toml`: the three C functions a program allocates
//! with and the library that defines them (MEM-08, ADR-0043).

#[cfg(test)]
mod tests;

use toml_edit::Item;
use typy_diagnostics::RuleId;

use super::{Problem, listed, shown};
use crate::settings::Allocator;

/// The functions, which the table names all three or none of.
const FUNCTIONS: [&str; 3] = ["allocate", "reallocate", "free"];

/// The keys the table may hold.
const KEYS: [&str; 4] = ["allocate", "reallocate", "free", "library"];

/// The longest name the table takes.
const MAX_NAME: usize = 255;

/// The prefixes of TyPy's own symbols: the runtime's, the entry's and the program's
/// functions. A function the table names may not have one, so a hook never calls itself or
/// a function of the program.
const RESERVED_PREFIXES: [&str; 3] = ["typy_", "__typy", "_TY"];

fn problem(range: std::ops::Range<usize>, message: String, help: Option<String>) -> Problem {
    Problem {
        range,
        message,
        help,
        rule: RuleId::MEM_08,
    }
}

/// Returns the allocator the table names, or what is wrong with it.
pub(super) fn allocator(item: &Item) -> Result<Allocator, Problem> {
    let whole = item.span().unwrap_or(0..0);
    let Some(table) = item.as_table_like() else {
        return Err(problem(
            whole,
            format!("`allocator` is {}, not a table", shown(item)),
            Some(
                "write it as a table: `[allocator]`, then `allocate = \"name\"` and the other \
                 functions on lines of their own"
                    .to_owned(),
            ),
        ));
    };
    let mut functions: [Option<String>; 3] = [None, None, None];
    let mut library = None;
    for (key, value) in table.iter() {
        let range = value.span().unwrap_or(0..0);
        let Some(position) = KEYS.iter().position(|each| *each == key) else {
            let range = table
                .get_key_value(key)
                .and_then(|(key, _)| key.span())
                .unwrap_or(range);
            return Err(problem(
                range,
                format!("`[allocator]` has no setting `{key}`"),
                Some(format!("the settings are {}", listed(&KEYS))),
            ));
        };
        let Some(text) = value.as_str() else {
            return Err(problem(
                range,
                format!("`allocator.{key}` is {}, not a string", shown(value)),
                None,
            ));
        };
        let checked = if key == "library" {
            library_name(text).map(|name| library = Some(name))
        } else {
            function_name(key, text).map(|name| functions[position] = Some(name))
        };
        checked.map_err(|(message, help)| problem(range, message, help))?;
    }
    let missing: Vec<String> = FUNCTIONS
        .iter()
        .zip(&functions)
        .filter(|(_, name)| name.is_none())
        .map(|(function, _)| format!("`{function}`"))
        .collect();
    let [Some(allocate), Some(reallocate), Some(free)] = functions else {
        let missing: Vec<&str> = missing.iter().map(String::as_str).collect();
        return Err(problem(
            whole,
            format!("`[allocator]` is missing {}", listed(&missing)),
            Some("name all three functions: allocate, reallocate and free".to_owned()),
        ));
    };
    // One C function cannot take the place of two, whose signatures differ.
    let shared = [
        (&allocate, &reallocate),
        (&allocate, &free),
        (&reallocate, &free),
    ]
    .into_iter()
    .find(|(left, right)| left == right);
    if let Some((name, _)) = shared {
        return Err(problem(
            whole,
            format!("`[allocator]` names `{name}` for two of its functions"),
            Some("allocate, reallocate and free are three different C functions".to_owned()),
        ));
    }
    Ok(Allocator {
        allocate,
        reallocate,
        free,
        library,
    })
}

/// What is wrong with a value: the message and the help.
type Wrong = (String, Option<String>);

/// Checks the name of a C function: an identifier no longer than `MAX_NAME` bytes, outside
/// TyPy's own prefix.
fn function_name(key: &str, text: &str) -> Result<String, Wrong> {
    let mut characters = text.chars();
    let identifier = text.len() <= MAX_NAME
        && characters
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|each| each.is_ascii_alphanumeric() || each == '_');
    if !identifier {
        return Err((
            format!("`allocator.{key}` is \"{text}\", which is not the name of a C function"),
            Some(format!(
                "name a C function, such as `mi_malloc`, of at most {MAX_NAME} letters, digits \
                 and underscores"
            )),
        ));
    }
    if let Some(prefix) = RESERVED_PREFIXES
        .iter()
        .find(|prefix| text.starts_with(**prefix))
    {
        return Err((
            format!(
                "`allocator.{key}` names `{text}`, but names that start with `{prefix}` \
                 belong to TyPy"
            ),
            None,
        ));
    }
    Ok(text.to_owned())
}

/// Checks a library name: one the linker takes as `-lname`, never read as an option.
fn library_name(text: &str) -> Result<String, Wrong> {
    let mut characters = text.chars();
    let name = text.len() <= MAX_NAME
        && characters
            .next()
            .is_some_and(|first| first.is_ascii_alphanumeric() || first == '_')
        && characters.all(|each| each.is_ascii_alphanumeric() || "_.+-".contains(each));
    if name {
        return Ok(text.to_owned());
    }
    Err((
        format!("`allocator.library` is \"{text}\", which is not the name of a library"),
        Some(
            "name the library as `-l` takes it, such as `mimalloc` for `libmimalloc.so`".to_owned(),
        ),
    ))
}
