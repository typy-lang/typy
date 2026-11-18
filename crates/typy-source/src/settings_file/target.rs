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
mod tests {
    use camino::Utf8PathBuf;

    use super::program_name;
    use crate::settings::{ProjectSettings, TargetTools};
    use crate::settings_file::{SETTINGS_FILE, SettingOverrides};

    /// Returns the settings `text` gives, or the message and rule of the first problem.
    fn read(text: &str) -> Result<ProjectSettings, (String, String)> {
        let directory = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
        std::fs::write(root.join(SETTINGS_FILE), text).unwrap();
        ProjectSettings::read(&root, SettingOverrides::default()).map_err(|failure| {
            let diagnostic = failure.diagnostics.iter().next().unwrap();
            (
                diagnostic.message().to_owned(),
                diagnostic
                    .rule()
                    .map(|rule| rule.as_str().to_owned())
                    .unwrap_or_default(),
            )
        })
    }

    #[test]
    fn a_program_is_a_name_or_an_absolute_path_never_an_option() {
        for name in [
            "cc",
            "aarch64-linux-gnu-gcc",
            "x86_64-linux-gnu-gcc-14",
            "/opt/cross/bin/aarch64-linux-gnu-ar",
            "clang++",
        ] {
            assert!(program_name(name), "{name}");
        }
        for name in ["", "-fuse-ld=lld", "bin/cc", "../cc", "my cc", "cc;rm"] {
            assert!(!program_name(name), "{name}");
        }
    }

    #[test]
    fn a_target_table_names_the_tools_that_build_for_it() {
        let settings = read(
            "[target.\"aarch64-unknown-linux-gnu\"]\nlinker = \"/opt/cross/bin/gcc\"\n\
             ar = \"llvm-ar\"\nobjcopy = \"llvm-objcopy\"\n\n\
             [target.\"x86_64-unknown-linux-gnu\"]\nlinker = \"clang\"\n",
        )
        .unwrap();
        assert_eq!(
            settings.targets.get("aarch64-unknown-linux-gnu"),
            Some(&TargetTools {
                linker: Some("/opt/cross/bin/gcc".to_owned()),
                archiver: Some("llvm-ar".to_owned()),
                object_copier: Some("llvm-objcopy".to_owned()),
            })
        );
        assert_eq!(
            settings.targets.get("x86_64-unknown-linux-gnu"),
            Some(&TargetTools {
                linker: Some("clang".to_owned()),
                ..TargetTools::default()
            })
        );
    }

    #[test]
    fn a_target_table_that_is_not_valid_is_e0107_of_cli_14() {
        for (text, message) in [
            ("target = 3\n", "`target` is an integer, not a table"),
            (
                "[target.\"x86_64-apple-darwin\"]\nlinker = \"cc\"\n",
                "`x86_64-apple-darwin` is not a target TyPy builds for",
            ),
            (
                "[target]\n\"aarch64-unknown-linux-gnu\" = \"cc\"\n",
                "`target.\"aarch64-unknown-linux-gnu\"` is the string \"cc\", not a table of tools",
            ),
            (
                "[target.\"aarch64-unknown-linux-gnu\"]\nld = \"ld.lld\"\n",
                "`target.\"aarch64-unknown-linux-gnu\"` has no tool `ld`",
            ),
            (
                "[target.\"aarch64-unknown-linux-gnu\"]\nlinker = \"bin/cc\"\n",
                "`target.\"aarch64-unknown-linux-gnu\".linker` is the string \"bin/cc\", not \
                 the name or absolute path of a program",
            ),
        ] {
            assert_eq!(
                read(text).unwrap_err(),
                (message.to_owned(), "CLI-14".to_owned()),
                "{text}"
            );
        }
    }
}
