//! A loaded program: its settings, its source files and its parsed modules.

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use typy_diagnostics::{
    Diagnostic, DiagnosticCode, Diagnostics, FileId, Position, RuleId, SourceLookup, Span,
    TextOffset,
};

use crate::loader::{
    Import, Location, embedded, imports, initialisation_order, is_standard, locate, not_found,
};
use crate::main_source::{CODE_PATH, MainSource};
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
        Self::load_main(&MainSource::File(main_path.to_owned()), settings)
    }

    /// Reads and parses the main module from `main`, relative to `settings.root`, and every
    /// module its imports reach (MOD-01, MOD-02). A module that `-m` names runs as `__main__`
    /// after the packages that hold it, and code given with `-c` is the file `<string>`
    /// (CLI-28). Returns every diagnostic of every file together.
    pub fn load_main(main: &MainSource, settings: ProjectSettings) -> Result<Self, LoadFailure> {
        let refused = |diagnostic: Box<Diagnostic>| {
            LoadFailure::new(Diagnostics::from(*diagnostic), Vec::new())
        };
        let mut loader = Loader {
            settings: &settings,
            files: Vec::new(),
            modules: BTreeMap::new(),
            imports: BTreeMap::new(),
            standard: BTreeSet::new(),
            diagnostics: Diagnostics::new(),
            pending: Vec::new(),
            main_code: None,
        };
        let (main, location, spec_name) = match main {
            MainSource::File(path) => {
                let relative = path.strip_prefix(&settings.root).unwrap_or(path).to_owned();
                let name = if settings.library {
                    library_module(&relative).map_err(refused)?
                } else {
                    ModuleName::main()
                };
                (name, file_location(relative), None)
            }
            MainSource::Code(code) => {
                loader.main_code = Some(code);
                (ModuleName::main(), file_location(CODE_PATH.into()), None)
            }
            MainSource::Module(name) => {
                let (spec_name, location) = locate_main(&settings, name).map_err(refused)?;
                (ModuleName::main(), location, Some(spec_name))
            }
        };
        if !loader.read(main.clone(), &location, None) {
            return Err(loader.failure());
        }
        if let Some(module) = loader.modules.get_mut(&main) {
            module.spec_name = spec_name;
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
    /// The main module's code when `-c` gives it, so no file is read for it (CLI-28).
    main_code: Option<&'settings str>,
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
            let (file, read_error) = match (importer, self.main_code, location.embedded) {
                (None, Some(code), _) | (_, _, Some(code)) => {
                    (SourceFile::from_text(id, location.path.clone(), code), None)
                }
                _ => SourceFile::read(id, &self.settings.root, &location.path),
            };
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
                        spec_name: None,
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
        let module = &self.modules[name];
        let (mut imports, errors) = imports(module);
        // The packages that hold the module `-m` runs are imported before it (CLI-28).
        let start = Span::empty(module.file, TextOffset(0));
        let packages = module
            .enclosing_packages()
            .into_iter()
            .map(|package| Import {
                module: package,
                span: start,
                at_module_level: true,
                optional: false,
            });
        imports.splice(0..0, packages);
        for error in errors {
            self.diagnostics.push(error);
        }
        for import in &imports {
            if self.modules.contains_key(&import.module) || self.standard.contains(&import.module) {
                continue;
            }
            // A module of the program comes first, as CPython's search finds the script's
            // directory first; then a standard library module TyPy gives in Python (LIB-27).
            if let Some(location) =
                locate(self.settings, &import.module).or_else(|| embedded(&import.module))
            {
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

/// Returns the location of a main file that holds no package.
const fn file_location(path: camino::Utf8PathBuf) -> Location {
    Location {
        path,
        is_package: false,
        embedded: None,
    }
}

/// Returns the name and the location of the module `-m` runs: the module `name`, or the
/// `__main__` module of the package `name`, as `python -m` runs them (CLI-28).
fn locate_main(
    settings: &ProjectSettings,
    name: &ModuleName,
) -> Result<(ModuleName, Location), Box<Diagnostic>> {
    let missing = |message: String| {
        Box::new(
            Diagnostic::without_location(DiagnosticCode::error(206), message)
                .with_rule(RuleId::LIB_08)
                .with_help(
                    "`-m` runs a module of the project in the current directory, a `.py` file \
                     or a package with a `__main__.py`",
                ),
        )
    };
    let location =
        locate(settings, name).ok_or_else(|| missing(format!("no module named `{name}`")))?;
    if !location.is_package {
        return Ok((name.clone(), location));
    }
    let main = name.child("__main__");
    match locate(settings, &main) {
        Some(location) if !location.is_package => Ok((main, location)),
        _ => Err(missing(format!(
            "no module named `{main}`; `{name}` is a package and cannot be run directly"
        ))),
    }
}

/// Returns the name of a library's module: its file's name without `.py`, which also names
/// the library and its C header, so it is an ASCII identifier (CLI-09, ADR-0044).
fn library_module(path: &Utf8Path) -> Result<ModuleName, Box<Diagnostic>> {
    let stem = path.file_stem().unwrap_or_default();
    let mut characters = stem.chars();
    let identifier = characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|each| each.is_ascii_alphanumeric() || each == '_');
    let refused = || {
        Box::new(Diagnostic::without_location(
            DiagnosticCode::error(429),
            format!(
                "a library's module is named after its file, and `{stem}` is not an ASCII \
                 identifier"
            ),
        )
        .with_rule(RuleId::CLI_09)
        .with_help("rename the file to a name of letters, digits and underscores, such as `my_tools.py`"))
    };
    if !identifier {
        return Err(refused());
    }
    ModuleName::new(stem).map_err(|_| refused())
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
mod tests;
