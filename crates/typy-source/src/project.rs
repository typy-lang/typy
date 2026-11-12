//! A loaded program: its settings, its source files and its parsed modules.

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use typy_diagnostics::{Diagnostics, FileId, Position, SourceLookup, Span, TextOffset};

use crate::loader::{
    Import, Location, imports, initialisation_order, is_standard, locate, not_found,
};
use crate::module::{Module, ModuleName};
use crate::parse::parse_with_warnings;
use crate::settings::ProjectSettings;
use crate::source_file::SourceFile;

/// The program being built: settings, files, modules, and the imports between them.
#[derive(Clone, Debug)]
pub struct Project {
    settings: ProjectSettings,
    main: ModuleName,
    modules: BTreeMap<ModuleName, Module>,
    files: Vec<SourceFile>,
    imports: BTreeMap<ModuleName, Vec<Import>>,
    order: Vec<ModuleName>,
    standard: BTreeSet<ModuleName>,
    warnings: Diagnostics,
}

/// Why a project did not load: the diagnostics, and the files they point into.
#[derive(Clone, Debug)]
pub struct LoadFailure {
    /// Every diagnostic of every file.
    pub diagnostics: Diagnostics,
    files: Vec<SourceFile>,
}

impl Project {
    /// Reads and parses the main file, `main_path`, relative to `settings.root`, and every
    /// module its imports reach, found under the root (MOD-01, MOD-02); returns every
    /// diagnostic of every file together.
    pub fn load(main_path: &Utf8Path, settings: ProjectSettings) -> Result<Self, LoadFailure> {
        let relative = main_path
            .strip_prefix(&settings.root)
            .unwrap_or(main_path)
            .to_owned();
        let mut loader = Loader {
            settings: &settings,
            files: Vec::new(),
            modules: BTreeMap::new(),
            imports: BTreeMap::new(),
            standard: BTreeSet::new(),
            diagnostics: Diagnostics::new(),
            pending: Vec::new(),
        };
        let main = ModuleName::main();
        let location = Location {
            path: relative,
            is_package: false,
        };
        if !loader.read(main.clone(), &location, None) {
            return Err(loader.failure());
        }
        while let Some(name) = loader.pending.pop() {
            loader.follow_imports(&name);
        }
        let (order, cycles) = initialisation_order(&loader.imports);
        for cycle in cycles {
            loader.diagnostics.push(cycle);
        }
        if loader.diagnostics.has_errors() {
            return Err(loader.failure());
        }
        let Loader {
            files,
            modules,
            imports,
            standard,
            diagnostics,
            ..
        } = loader;
        Ok(Self {
            settings,
            main,
            modules,
            files,
            imports,
            order,
            standard,
            warnings: diagnostics,
        })
    }

    /// Returns the warnings of reading and parsing the program's files (DIA-08).
    #[must_use]
    pub const fn warnings(&self) -> &Diagnostics {
        &self.warnings
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

    /// Returns a module of the program by its name.
    #[must_use]
    pub fn module(&self, name: &ModuleName) -> Option<&Module> {
        self.modules.get(name)
    }

    /// Returns the modules with each one's module-level imports before it, the order in which
    /// they are checked.
    #[must_use]
    pub fn order(&self) -> &[ModuleName] {
        &self.order
    }

    /// Returns the modules in the order of `order`.
    pub fn modules_in_order(&self) -> impl Iterator<Item = &Module> {
        self.order.iter().filter_map(|name| self.modules.get(name))
    }

    /// Returns the modules a module's imports need, in source order.
    #[must_use]
    pub fn imports(&self, name: &ModuleName) -> &[Import] {
        self.imports.get(name).map_or(&[], Vec::as_slice)
    }

    /// Returns the standard library modules the program imports, which no file provides.
    pub fn standard_modules(&self) -> impl Iterator<Item = &ModuleName> {
        self.standard.iter()
    }

    /// Returns whether a module the program imports is a standard library module, one that
    /// no file under the root provides.
    #[must_use]
    pub fn imports_standard(&self, name: &ModuleName) -> bool {
        self.standard.contains(name)
    }

    /// Returns a file by its ID.
    #[must_use]
    pub fn file(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0 as usize)
    }
}

/// The state of a load: the files and modules read so far, and those whose imports are
/// still to be followed.
struct Loader<'settings> {
    settings: &'settings ProjectSettings,
    files: Vec<SourceFile>,
    modules: BTreeMap<ModuleName, Module>,
    imports: BTreeMap<ModuleName, Vec<Import>>,
    standard: BTreeSet<ModuleName>,
    diagnostics: Diagnostics,
    pending: Vec<ModuleName>,
}

impl Loader<'_> {
    /// Reads and parses a module from its location, reusing a file already read, as for the
    /// main file imported by its own name; returns false when it cannot be read or parsed.
    fn read(&mut self, name: ModuleName, location: &Location, importer: Option<Span>) -> bool {
        let existing = self
            .files
            .iter()
            .find(|file| file.path() == location.path)
            .map(SourceFile::id);
        let mut unreadable = false;
        let id = existing.unwrap_or_else(|| {
            let id = FileId(u32::try_from(self.files.len()).unwrap_or(u32::MAX));
            let (file, read_error) = SourceFile::read(id, &self.settings.root, &location.path);
            self.files.push(file);
            if let Some(mut diagnostic) = read_error {
                if let Some(span) = importer {
                    diagnostic = diagnostic.with_label(span, "imported here");
                }
                self.diagnostics.push(diagnostic);
                unreadable = true;
            }
            id
        });
        // A file that cannot be read has no text to parse, so it gives one error.
        if unreadable {
            return false;
        }
        let file = &self.files[id.0 as usize];
        match parse_with_warnings(file, self.settings.python_floor) {
            Ok(parsed) => {
                for warning in parsed.warnings {
                    self.diagnostics.push(warning);
                }
                let syntax = parsed.tree;
                self.modules.insert(
                    name.clone(),
                    Module {
                        name: name.clone(),
                        file: id,
                        syntax,
                        is_package: location.is_package,
                    },
                );
                self.pending.push(name);
                true
            }
            // A file read before parsed then, so only a new file can fail to parse here.
            Err(errors) => {
                for error in errors {
                    self.diagnostics.push(error);
                }
                false
            }
        }
    }

    /// Loads each module the imports of `name` need that is not loaded yet.
    fn follow_imports(&mut self, name: &ModuleName) {
        let (imports, errors) = imports(&self.modules[name]);
        for error in errors {
            self.diagnostics.push(error);
        }
        for import in &imports {
            if self.modules.contains_key(&import.module) || self.standard.contains(&import.module) {
                continue;
            }
            if let Some(location) = locate(self.settings, &import.module) {
                self.read(import.module.clone(), &location, Some(import.span));
            } else if is_standard(&import.module) {
                // The checker says whether TyPy provides it (LIB-08).
                self.standard.insert(import.module.clone());
            } else if !import.optional {
                let message = match import.module.parent() {
                    Some(parent)
                        if self
                            .modules
                            .get(&parent)
                            .is_some_and(|module| !module.is_package) =>
                    {
                        format!(
                            "no module named `{}`; `{parent}` is not a package",
                            import.module
                        )
                    }
                    _ => format!("no module named `{}`", import.module),
                };
                self.diagnostics.push(not_found(import.span, message));
            }
        }
        self.imports.insert(name.clone(), imports);
    }

    fn failure(self) -> LoadFailure {
        LoadFailure::new(self.diagnostics, self.files)
    }
}

impl LoadFailure {
    /// Returns a failure with `diagnostics` that point into `files`.
    pub(crate) const fn new(diagnostics: Diagnostics, files: Vec<SourceFile>) -> Self {
        Self { diagnostics, files }
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
        assert_eq!(
            failure.path(diagnostics[0].span().unwrap().file()),
            "absent.py"
        );
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
