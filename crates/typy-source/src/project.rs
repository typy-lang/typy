//! A loaded program: its settings, its source files and its parsed modules.

use std::collections::BTreeMap;

use camino::Utf8Path;
use typy_diagnostics::{Diagnostics, FileId, Position, SourceLookup, TextOffset};

use crate::module::{Module, ModuleName};
use crate::parse::parse;
use crate::settings::ProjectSettings;
use crate::source_file::SourceFile;

/// The program being built: settings, files and modules.
#[derive(Clone, Debug)]
pub struct Project {
    settings: ProjectSettings,
    main: ModuleName,
    modules: BTreeMap<ModuleName, Module>,
    files: Vec<SourceFile>,
}

/// Why a project did not load: the diagnostics, and the files they point into.
#[derive(Clone, Debug)]
pub struct LoadFailure {
    /// Every diagnostic of every file.
    pub diagnostics: Diagnostics,
    files: Vec<SourceFile>,
}

impl Project {
    /// Reads and parses the main file, `main_path`, relative to `settings.root`; returns every diagnostic of every file together.
    pub fn load(main_path: &Utf8Path, settings: ProjectSettings) -> Result<Self, LoadFailure> {
        let relative = main_path
            .strip_prefix(&settings.root)
            .unwrap_or(main_path)
            .to_owned();
        let (file, read_error) = SourceFile::read(FileId(0), &settings.root, &relative);
        let mut diagnostics = Diagnostics::new();
        if let Some(diagnostic) = read_error {
            diagnostics.push(diagnostic);
            return Err(LoadFailure {
                diagnostics,
                files: vec![file],
            });
        }
        match parse(&file, settings.python_floor) {
            Ok(syntax) => {
                let main = ModuleName::main();
                let module = Module {
                    name: main.clone(),
                    file: file.id(),
                    syntax,
                };
                Ok(Self {
                    settings,
                    modules: BTreeMap::from([(main.clone(), module)]),
                    main,
                    files: vec![file],
                })
            }
            Err(errors) => {
                for error in errors {
                    diagnostics.push(error);
                }
                Err(LoadFailure {
                    diagnostics,
                    files: vec![file],
                })
            }
        }
    }

    /// Returns the settings.
    #[must_use]
    pub fn settings(&self) -> &ProjectSettings {
        &self.settings
    }

    /// Returns the main module.
    #[must_use]
    pub fn main_module(&self) -> &Module {
        &self.modules[&self.main]
    }

    /// Returns every module, in name order.
    pub fn modules(&self) -> impl Iterator<Item = &Module> {
        self.modules.values()
    }

    /// Returns a file by its ID.
    #[must_use]
    pub fn file(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0 as usize)
    }
}

fn find(files: &[SourceFile], id: FileId) -> Option<&SourceFile> {
    files.get(id.0 as usize)
}

fn path(files: &[SourceFile], id: FileId) -> &str {
    find(files, id).map_or("<unknown>", |file| file.path().as_str())
}

fn position(files: &[SourceFile], id: FileId, at: TextOffset) -> Position {
    find(files, id).map_or(Position { line: 1, column: 1 }, |file| file.position(at))
}

fn line_text(files: &[SourceFile], id: FileId, line: u32) -> &str {
    find(files, id).map_or("", |file| file.lines().line_text(file.text(), line))
}

impl SourceLookup for Project {
    fn path(&self, file: FileId) -> &str {
        path(&self.files, file)
    }
    fn position(&self, file: FileId, offset: TextOffset) -> Position {
        position(&self.files, file, offset)
    }
    fn line_text(&self, file: FileId, line: u32) -> &str {
        line_text(&self.files, file, line)
    }
}

impl SourceLookup for LoadFailure {
    fn path(&self, file: FileId) -> &str {
        path(&self.files, file)
    }
    fn position(&self, file: FileId, offset: TextOffset) -> Position {
        position(&self.files, file, offset)
    }
    fn line_text(&self, file: FileId, line: u32) -> &str {
        line_text(&self.files, file, line)
    }
}

#[cfg(test)]
mod tests {
    use super::Project;
    use crate::settings::ProjectSettings;
    use crate::syntax::Stmt;
    use camino::Utf8PathBuf;
    use typy_diagnostics::{SourceLookup, render_text};

    fn project_with(source: &str) -> (tempfile::TempDir, Utf8PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("app.py"), source).unwrap();
        let root = Utf8PathBuf::from_path_buf(directory.path().to_path_buf()).unwrap();
        (directory, root)
    }

    #[test]
    fn valid_program_loads_as_the_main_module() {
        let (_directory, root) = project_with("print(1 + 2)\n");
        let project =
            Project::load(&root.join("app.py"), ProjectSettings::new(root.clone())).unwrap();
        let main = project.main_module();
        assert_eq!(main.name.as_str(), "__main__");
        assert!(matches!(main.syntax.body.as_slice(), [Stmt::Expr { .. }]));
        assert_eq!(project.modules().count(), 1);
        assert_eq!(project.path(main.file), "app.py");
        assert_eq!(project.line_text(main.file, 1), "print(1 + 2)");
        assert_eq!(project.settings().root, root);
        assert!(project.file(main.file).is_some());
    }

    #[test]
    fn syntax_errors_are_returned_with_the_files_to_render_them() {
        let (_directory, root) = project_with("x = (1 +\n");
        let failure = Project::load(&root.join("app.py"), ProjectSettings::new(root)).unwrap_err();
        assert!(failure.diagnostics.has_errors());
        let first = failure.diagnostics.clone().into_sorted(&failure).remove(0);
        let text = render_text(&first, &failure);
        assert!(text.starts_with("error[E0103]: "), "{text}");
        assert!(text.contains("--> app.py:"), "{text}");
    }

    #[test]
    fn missing_main_file_fails_with_e0105() {
        let directory = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(directory.path().to_path_buf()).unwrap();
        let failure =
            Project::load(&root.join("absent.py"), ProjectSettings::new(root)).unwrap_err();
        let diagnostics = failure.diagnostics.clone().into_sorted(&failure);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code().to_string(), "E0105");
        assert_eq!(failure.path(diagnostics[0].span().file()), "absent.py");
    }

    #[test]
    fn unknown_file_ids_render_without_panicking() {
        let (_directory, root) = project_with("pass\n");
        let project = Project::load(&root.join("app.py"), ProjectSettings::new(root)).unwrap();
        let unknown = typy_diagnostics::FileId(99);
        assert_eq!(project.path(unknown), "<unknown>");
        assert_eq!(project.line_text(unknown, 1), "");
        assert_eq!(
            project
                .position(unknown, typy_diagnostics::TextOffset(0))
                .line,
            1
        );
    }
}
