//! Reads the `[target."TRIPLE"]` tables of `typy.toml`: the C tools that build for a target in
//! place of those TyPy would run (CLI-14, ADR-0051).

use std::collections::BTreeMap;

use toml_edit::Item;
use typy_diagnostics::RuleId;

use super::{Problem, listed, shown};
use crate::settings::TargetTools;

/// The targets TyPy builds for, as `typy.toml` names them. The code generator's triples are
/// the same, which a test of the toolchain checks.
pub const TARGET_TRIPLES: [&str; 4] = [
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-gnu",
    "aarch64-apple-darwin",
];

/// The keys of a target's table.
const TOOLS: [&str; 3] = ["linker", "ar", "objcopy"];

fn problem(range: std::ops::Range<usize>, message: String, help: String) -> Problem {
    Problem {
        range,
        message,
        help: Some(help),
        rule: RuleId::CLI_14,
    }
}

/// Returns the tools each target's table names, or what is wrong with the tables.
pub(super) fn targets(item: &Item) -> Result<BTreeMap<String, TargetTools>, Problem> {
    let Some(table) = item.as_table_like() else {
        return Err(problem(
            item.span().unwrap_or(0..0),
            format!("`target` is {}, not a table", shown(item)),
            "write a table for each target, such as `[target.\"aarch64-unknown-linux-gnu\"]` \
             and `linker = \"aarch64-linux-gnu-gcc\"`"
                .to_owned(),
        ));
    };
    let mut targets = BTreeMap::new();
    for (triple, value) in table.iter() {
        let range = table
            .get_key_value(triple)
            .and_then(|(key, _)| key.span())
            .or_else(|| value.span())
            .unwrap_or(0..0);
        if !TARGET_TRIPLES.contains(&triple) {
            return Err(problem(
                range,
                format!("`{triple}` is not a target TyPy builds for"),
                format!("the targets are {}", listed(&TARGET_TRIPLES)),
            ));
        }
        targets.insert(triple.to_owned(), tools(triple, value)?);
    }
    Ok(targets)
}

/// Returns the tools one target's table names.
fn tools(triple: &str, value: &Item) -> Result<TargetTools, Problem> {
    let Some(entry) = value.as_table_like() else {
        return Err(problem(
            value.span().unwrap_or(0..0),
            format!(
                "`target.\"{triple}\"` is {}, not a table of tools",
                shown(value)
            ),
            format!("name any of {}, as `linker = \"cc\"`", listed(&TOOLS)),
        ));
    };
    let mut tools = TargetTools::default();
    for (key, tool) in entry.iter() {
        let range = tool.span().unwrap_or(0..0);
        let slot = match key {
            "linker" => &mut tools.linker,
            "ar" => &mut tools.archiver,
            "objcopy" => &mut tools.object_copier,
            _ => {
                return Err(problem(
                    entry
                        .get_key_value(key)
                        .and_then(|(name, _)| name.span())
                        .unwrap_or(range),
                    format!("`target.\"{triple}\"` has no tool `{key}`"),
                    format!("the tools are {}", listed(&TOOLS)),
                ));
            }
        };
        let Some(name) = tool.as_str().filter(|name| program_name(name)) else {
            return Err(problem(
                range,
                format!(
                    "`target.\"{triple}\".{key}` is {}, not the name or absolute path of a \
                     program",
                    shown(tool)
                ),
                "write the program's name, such as `aarch64-linux-gnu-gcc`, which is looked \
                 up on `PATH`, or its absolute path"
                    .to_owned(),
            ));
        };
        *slot = Some(name.to_owned());
    }
    Ok(tools)
}

/// Returns whether `name` is a program TyPy may run: a name looked up on `PATH` or an absolute
/// path, which is never read as an option and never passed to a shell.
fn program_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && (!name.contains('/') || name.starts_with('/'))
        && name
            .chars()
            .all(|each| each.is_ascii_alphanumeric() || "_.+-/".contains(each))
}

#[cfg(test)]
mod tests;
