//! Reads `typy.toml`, the project's settings, once when a command starts; a flag on the
//! command line wins over the file (CLI-07, CLI-12, UC-11).

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use toml_edit::{Document, Item};
use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, FileId, RuleId, Span, TextOffset};

use crate::project::LoadFailure;
use crate::settings::{ProjectSettings, PythonFloor};
use crate::source_file::SourceFile;

/// The name of the settings file, in the directory of the main file.
pub const SETTINGS_FILE: &str = "typy.toml";

/// The keys `typy.toml` may hold.
const KEYS: [&str; 2] = ["python-floor", "module-paths"];

/// The settings flags on the command line give, which win over `typy.toml` (CLI-07).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettingOverrides {
    /// `--python-floor`.
    pub python_floor: Option<PythonFloor>,
}

impl ProjectSettings {
    /// Returns the settings of the project rooted at `root`: the defaults, then `typy.toml`
    /// when the root has one, then `overrides`. A file that is not valid fails with E0107 for
    /// each thing wrong in it.
    pub fn read(root: &Utf8Path, overrides: SettingOverrides) -> Result<Self, LoadFailure> {
        let mut settings = Self::new(root.to_owned());
        let path = root.join(SETTINGS_FILE);
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let file = SourceFile::from_text(FileId(0), SETTINGS_FILE.into(), &text);
                let diagnostics = settings.apply(&text, &file);
                if diagnostics.has_errors() {
                    return Err(LoadFailure::new(diagnostics, vec![file]));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                let diagnostic = Diagnostic::without_location(
                    DiagnosticCode::error(107),
                    format!("cannot read {path}: {error}"),
                )
                .with_rule(RuleId::CLI_07);
                return Err(LoadFailure::new(Diagnostics::from(diagnostic), Vec::new()));
            }
        }
        if let Some(floor) = overrides.python_floor {
            settings.python_floor = floor;
        }
        Ok(settings)
    }

    /// Applies the settings `text` holds and returns what is wrong in it.
    fn apply(&mut self, text: &str, file: &SourceFile) -> Diagnostics {
        let mut diagnostics = Diagnostics::new();
        let document = match Document::parse(text) {
            Ok(document) => document,
            Err(error) => {
                let problem = Problem {
                    range: error.span().unwrap_or(0..0),
                    message: format!(
                        "`typy.toml` is not valid TOML: {}",
                        error.message().trim_end()
                    ),
                    help: None,
                    rule: RuleId::CLI_07,
                };
                diagnostics.push(problem.diagnostic(file));
                return diagnostics;
            }
        };
        for (key, item) in document.iter() {
            let found = match key {
                "python-floor" => self.python_floor(item),
                "module-paths" => self.module_paths(item),
                _ => Err(Problem {
                    range: document
                        .as_table()
                        .key(key)
                        .and_then(toml_edit::Key::span)
                        .unwrap_or(0..0),
                    message: format!("`typy.toml` has no setting `{key}`"),
                    help: Some(format!("the settings are {}", KEYS.join(" and "))),
                    rule: RuleId::CLI_07,
                }),
            };
            if let Err(problem) = found {
                diagnostics.push(problem.diagnostic(file));
            }
        }
        diagnostics
    }

    /// Reads `python-floor`, a string naming one of the floors (CLI-12).
    fn python_floor(&mut self, item: &Item) -> Result<(), Problem> {
        let floor = item.as_str().and_then(PythonFloor::parse).ok_or_else(|| {
            let floors: Vec<&str> = PythonFloor::ALL
                .iter()
                .map(|floor| floor.as_str())
                .collect();
            Problem {
                range: item.span().unwrap_or(0..0),
                message: format!(
                    "`python-floor` is {}, not one of the strings {}",
                    shown(item),
                    floors.join(", ")
                ),
                help: Some("write it as a string, such as `python-floor = \"3.15\"`".to_owned()),
                rule: RuleId::CLI_12,
            }
        })?;
        self.python_floor = floor;
        Ok(())
    }

    /// Reads `module-paths`, an array of directories under the root (MOD-01).
    fn module_paths(&mut self, item: &Item) -> Result<(), Problem> {
        let problem = |range, message, help| Problem {
            range,
            message,
            help,
            rule: RuleId::CLI_07,
        };
        let Some(array) = item.as_array() else {
            return Err(problem(
                item.span().unwrap_or(0..0),
                format!("`module-paths` is {}, not an array of strings", shown(item)),
                Some("write it as an array, such as `module-paths = [\"lib\"]`".to_owned()),
            ));
        };
        let mut paths = Vec::new();
        for value in array {
            let range = value.span().unwrap_or(0..0);
            let Some(text) = value.as_str() else {
                let name = value.type_name();
                let message = format!("a module path is {} {name}, not a string", article(name));
                return Err(problem(range, message, None));
            };
            match self.module_path(text) {
                Ok(path) => paths.push(path),
                Err(message) => return Err(problem(range, message, None)),
            }
        }
        self.module_paths = paths;
        Ok(())
    }

    /// Checks one module path: a directory under the root, named relative to it, so the
    /// build never reads outside the project.
    fn module_path(&self, text: &str) -> Result<Utf8PathBuf, String> {
        let path = Utf8Path::new(text);
        let inside = !text.is_empty()
            && path
                .components()
                .all(|component| matches!(component, Utf8Component::Normal(_)));
        if !inside {
            return Err(format!(
                "the module path `{text}` is not a directory under the project root, named \
                 relative to it"
            ));
        }
        if !self.root.join(path).is_dir() {
            return Err(format!("the module path `{text}` is not a directory"));
        }
        Ok(path.to_owned())
    }
}

/// What is wrong with one setting.
struct Problem {
    /// Where it is in `typy.toml`, in bytes.
    range: std::ops::Range<usize>,
    message: String,
    help: Option<String>,
    /// The requirement of the setting.
    rule: RuleId,
}

impl Problem {
    /// Returns E0107 at the problem's place in `typy.toml`.
    fn diagnostic(self, file: &SourceFile) -> Diagnostic {
        let offset = |at: usize| TextOffset(u32::try_from(at).unwrap_or(u32::MAX));
        let code = DiagnosticCode::error(107);
        let diagnostic =
            match Span::new(file.id(), offset(self.range.start), offset(self.range.end)) {
                Some(span) => Diagnostic::new(code, span, self.message),
                None => Diagnostic::without_location(code, self.message),
            }
            .with_rule(self.rule);
        match self.help {
            Some(help) => diagnostic.with_help(help),
            None => diagnostic,
        }
    }
}

/// Returns how a wrong value is named in a message: a string with its text, else its type.
fn shown(item: &Item) -> String {
    item.as_str().map_or_else(
        || format!("{} {}", article(item.type_name()), item.type_name()),
        |text| format!("the string \"{text}\""),
    )
}

/// Returns `an` before a TOML type name that starts with a vowel, and `a` otherwise.
fn article(name: &str) -> &'static str {
    if name.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    }
}

#[cfg(test)]
mod tests {
    use super::{SETTINGS_FILE, SettingOverrides};
    use crate::settings::{ProjectSettings, PythonFloor};
    use camino::Utf8PathBuf;

    fn root(settings: Option<&str>) -> (tempfile::TempDir, Utf8PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
        if let Some(text) = settings {
            std::fs::write(root.join(SETTINGS_FILE), text).unwrap();
        }
        std::fs::create_dir(root.join("lib")).unwrap();
        (directory, root)
    }

    #[test]
    fn without_typy_toml_the_defaults_apply() {
        let (_guard, root) = root(None);
        let settings = ProjectSettings::read(&root, SettingOverrides::default()).unwrap();
        assert_eq!(settings, ProjectSettings::new(root));
    }

    #[test]
    fn each_setting_is_read_and_a_flag_wins_over_the_file() {
        let (_guard, root) = root(Some(
            "# The project's settings.\npython-floor = \"3.14\"\nmodule-paths = [\"lib\"]\n",
        ));
        let settings = ProjectSettings::read(&root, SettingOverrides::default()).unwrap();
        assert_eq!(settings.python_floor, PythonFloor::V3_14);
        assert_eq!(settings.module_paths, [Utf8PathBuf::from("lib")]);
        let overrides = SettingOverrides {
            python_floor: Some(PythonFloor::V3_15),
        };
        let settings = ProjectSettings::read(&root, overrides).unwrap();
        assert_eq!(settings.python_floor, PythonFloor::V3_15);
    }

    #[test]
    fn every_wrong_setting_is_reported_once() {
        let (_guard, root) = root(Some(
            "python-floor = 3\nmodule-paths = [\"/abs\", \"lib\"]\nother = true\n",
        ));
        let failure = ProjectSettings::read(&root, SettingOverrides::default()).unwrap_err();
        let codes: Vec<String> = failure
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code().to_string())
            .collect();
        assert_eq!(codes, ["E0107"; 3]);
    }

    #[test]
    fn a_settings_file_that_cannot_be_read_is_e0107() {
        let (_guard, root) = root(None);
        std::fs::create_dir(root.join(SETTINGS_FILE)).unwrap();
        let failure = ProjectSettings::read(&root, SettingOverrides::default()).unwrap_err();
        let diagnostic = failure.diagnostics.iter().next().unwrap();
        assert_eq!(diagnostic.code().to_string(), "E0107");
        assert!(diagnostic.message().starts_with("cannot read "));
    }
}
