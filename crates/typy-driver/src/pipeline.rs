//! Runs the stages of one build in order and stops after the stage `--emit` names.

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use typy_build::{
    Artifact, BuildRequest, EmitStage, LibraryKind, LibraryLayout, Linker, RuntimeArchive,
};
use typy_check::{TypeChecker, TypedProgram};
use typy_codegen::{Target, assembly, llvm_ir, object};
use typy_diagnostics::{Diagnostics, WarningLevels};
use typy_source::{Platform, Project, ProjectSettings, SETTINGS_FILE, SettingOverrides};
use typy_tyir::{TyirAllocator, TyirLibrary, TyirModule};

use crate::emission::Emission;
use crate::failure::Failure;
use crate::outcome::Outcome;
use crate::sources::Sources;
use crate::success::{Success, Warnings};

/// Runs builds and checks. It holds the compiler's location, which finds the runtime
/// archive, and the settings the command line gives, so one pipeline runs any number of
/// builds (detailed design section 12).
#[derive(Debug)]
pub struct Pipeline {
    compiler: Utf8PathBuf,
    overrides: SettingOverrides,
    /// The level of each warning code, from `-W`, `-A`, `-D` and `--deny-warnings` (CLI-27).
    warning_levels: WarningLevels,
    /// Whether each command of the C toolchain is written before it runs (CLI-23).
    verbose: bool,
    /// Whether the object file of a build is kept beside its output (CLI-24).
    save_temps: bool,
    /// How long each stage that ran took, in order, for `--timings` (CLI-22). A pipeline
    /// may run builds on several threads, as the conformance suite does, so it is locked.
    timings: Mutex<Vec<(&'static str, Duration)>>,
}

impl Clone for Pipeline {
    fn clone(&self) -> Self {
        Self {
            compiler: self.compiler.clone(),
            overrides: self.overrides,
            warning_levels: self.warning_levels.clone(),
            verbose: self.verbose,
            save_temps: self.save_temps,
            timings: Mutex::new(self.timings()),
        }
    }
}

impl Pipeline {
    /// Returns a pipeline for the compiler executable at `compiler`, whose runtime archives are
    /// in `<prefix>/lib/typy/` beside `<prefix>/bin/`.
    #[must_use]
    pub fn new(compiler: impl Into<Utf8PathBuf>) -> Self {
        Self {
            compiler: compiler.into(),
            overrides: SettingOverrides::default(),
            warning_levels: WarningLevels::default(),
            verbose: false,
            save_temps: false,
            timings: Mutex::new(Vec::new()),
        }
    }

    /// Returns the pipeline, writing each command of the C toolchain to standard error before
    /// it runs when `verbose` (CLI-23).
    #[must_use]
    pub const fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Returns the pipeline, keeping a build's object file beside its output when
    /// `save_temps` (CLI-24).
    #[must_use]
    pub const fn with_save_temps(mut self, save_temps: bool) -> Self {
        self.save_temps = save_temps;
        self
    }

    /// Returns whether each command `typy` runs is written to standard error first (CLI-23).
    #[must_use]
    pub const fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Returns how long each stage that ran took, in order, for `--timings` (CLI-22).
    #[must_use]
    pub fn timings(&self) -> Vec<(&'static str, Duration)> {
        self.timings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Records that `stage` took `time`.
    fn record(&self, stage: &'static str, time: Duration) {
        self.timings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((stage, time));
    }

    /// Runs `work` as the stage `stage`, and records how long it took.
    fn timed<T>(&self, stage: &'static str, work: impl FnOnce() -> T) -> T {
        let started = Instant::now();
        let result = work();
        self.record(stage, started.elapsed());
        result
    }

    /// Returns the pipeline with the level of each warning code: a denied warning fails a
    /// build or a check before any code is generated, and an allowed one is not reported
    /// (DIA-07, CLI-27).
    #[must_use]
    pub fn with_warning_levels(mut self, levels: WarningLevels) -> Self {
        self.warning_levels = levels;
        self
    }

    /// Returns the level of each warning code.
    #[must_use]
    pub const fn warning_levels(&self) -> &WarningLevels {
        &self.warning_levels
    }

    /// Returns the pipeline with settings that win over each project's `typy.toml` (CLI-07).
    #[must_use]
    pub const fn with_overrides(mut self, overrides: SettingOverrides) -> Self {
        self.overrides = overrides;
        self
    }

    /// Loads and checks the program whose main file is `main` for `target`, without generating
    /// code (CLI-03).
    pub fn check(&self, main: &Utf8Path, target: Target) -> Result<Success, Failure> {
        let project = self.timed("load", || self.load(main, false, target))?;
        let (_, warnings) = self.timed("check", || self.check_program(&project))?;
        Ok(Success {
            outcome: Outcome::Checked(Sources::Project(Box::new(project))),
            warnings,
        })
    }

    /// Returns the path from the root and the text of every file of the program whose main
    /// file is `main`, the main file first and `typy.toml` last, or `None` when it does not
    /// load.
    #[must_use]
    pub fn sources(&self, main: &Utf8Path) -> Option<Vec<(String, String)>> {
        use typy_diagnostics::SourceLookup as _;
        let project = self.load(main, false, Target::host()).ok()?;
        let mut files: Vec<(String, String)> = Vec::new();
        let modules = std::iter::once(project.main_module()).chain(project.modules());
        for (module, file) in
            modules.filter_map(|module| Some((module, project.file(module.file)?)))
        {
            let path = project.path(module.file).to_owned();
            if !files.iter().any(|(known, _)| *known == path) {
                files.push((path, file.text().to_owned()));
            }
        }
        if let Ok(settings) = std::fs::read_to_string(root_of(main).join(SETTINGS_FILE)) {
            files.push((SETTINGS_FILE.to_owned(), settings));
        }
        Some(files)
    }

    /// Runs the build `request` asks for: the output of one stage, or a linked executable.
    pub fn run(&self, request: &BuildRequest) -> Result<Success, Failure> {
        let (outcome, warnings) = if let Some(stage) = request.emit() {
            let (emission, warnings) = self.emit(stage, request)?;
            (Outcome::Emitted(emission), warnings)
        } else {
            let (artifact, warnings) = self.build(request)?;
            (Outcome::Executable(artifact), warnings)
        };
        Ok(Success { outcome, warnings })
    }

    /// Runs the stages up to `stage` and returns its output; each stage runs only when the
    /// stages before it found no errors.
    fn emit(
        &self,
        stage: EmitStage,
        request: &BuildRequest,
    ) -> Result<(Emission, Warnings), Failure> {
        let project = self.timed("load", || {
            self.load(
                request.main(),
                request.library().is_some(),
                request.target(),
            )
        })?;
        if stage == EmitStage::Ast {
            let warnings = self.warnings(project.warnings(), &project)?;
            return Ok((syntax_text(&project), warnings));
        }
        let (program, warnings) = self.timed("check", || self.check_program(&project))?;
        if stage == EmitStage::TypedAst {
            let text = typy_check::dump(&program, &project);
            return Ok((Emission::Text(text), warnings));
        }
        let module = self.timed("lower", || lower(&program, &project))?;
        let target = request.target();
        let options = request.codegen_options();
        let failed = |diagnostics| with_project(diagnostics, &project);
        let emission = self.timed("generate", || -> Result<Emission, Failure> {
            Ok(match stage {
                EmitStage::Ast => syntax_text(&project),
                EmitStage::TypedAst => Emission::Text(typy_check::dump(&program, &project)),
                EmitStage::Tyir => Emission::Text(typy_tyir::print_without_counting(&module)),
                EmitStage::TyirMem => Emission::Text(typy_tyir::print(&module)),
                EmitStage::LlvmIr => {
                    Emission::Text(llvm_ir(&module, target, &options, &project).map_err(failed)?)
                }
                EmitStage::Asm => {
                    Emission::Text(assembly(&module, target, &options, &project).map_err(failed)?)
                }
                EmitStage::Obj => Emission::Bytes(
                    object(&module, target, &options, &project)
                        .map_err(failed)?
                        .bytes,
                ),
            })
        })?;
        Ok((emission, warnings))
    }

    /// Builds and links the executable. The runtime archive is opened first, so a broken
    /// installation fails before any work.
    fn build(&self, request: &BuildRequest) -> Result<(Artifact, Warnings), Failure> {
        let archive = self.runtime_archive(request)?;
        let project = self.timed("load", || {
            self.load(
                request.main(),
                request.library().is_some(),
                request.target(),
            )
        })?;
        let (program, warnings) = self.timed("check", || self.check_program(&project))?;
        let module = self.timed("lower", || lower(&program, &project))?;
        let linker = linker(request, archive, &project, &module).with_verbose(self.verbose);
        let failed = |diagnostics| with_project(diagnostics, &project);
        let object = self
            .timed("generate", || {
                object(
                    &module,
                    request.target(),
                    &request.codegen_options(),
                    &project,
                )
            })
            .map_err(failed)?;
        let linking = Instant::now();
        let artifact = match (request.library(), &module.library) {
            (Some(kind), Some(library)) => {
                let layout = match request.output() {
                    Some(path) => LibraryLayout::at(path, kind),
                    None => LibraryLayout::beside(
                        request.main().parent().unwrap_or(Utf8Path::new("")),
                        kind,
                        &library.name,
                        library.version.as_deref(),
                        request.target().triple,
                    ),
                };
                let exports: Vec<String> = library
                    .exports
                    .iter()
                    .map(|export| export.name.clone())
                    .collect();
                let artifact = linker
                    .link_library(&object, &layout, kind, &exports)
                    .map_err(failed)?;
                write_header(library, &layout.path).map_err(failed)?;
                // CPython loads the shared library of the build's release, also for a static
                // build, whose binding serves CPython and TyPy alike (ADR-0046).
                let soname = match (kind, &layout.soname) {
                    (LibraryKind::Shared, Some(soname)) => soname.clone(),
                    _ => LibraryLayout::beside(
                        Utf8Path::new(""),
                        LibraryKind::Shared,
                        &library.name,
                        library.version.as_deref(),
                        request.target().triple,
                    )
                    .soname
                    .unwrap_or_default(),
                };
                write_binding(library, &layout.path, &soname).map_err(failed)?;
                artifact
            }
            _ => linker
                .link(&object, &request.output_path())
                .map_err(failed)?,
        };
        self.record("link", linking.elapsed());
        if self.save_temps {
            keep_object(&object.bytes, &artifact.path).map_err(Failure::without_sources)?;
        }
        Ok((artifact, warnings))
    }

    /// Checks the program, and returns it with every warning of loading and checking it.
    fn check_program(&self, project: &Project) -> Result<(TypedProgram, Warnings), Failure> {
        let program = TypeChecker::check(project)
            .map_err(|diagnostics| with_project(diagnostics, project))?;
        let mut found = project.warnings().clone();
        found.extend(program.warnings.clone());
        let warnings = self.warnings(&found, project)?;
        Ok((program, warnings))
    }

    /// Returns the warnings at their levels, or a failure when any is denied; the failure
    /// reports the denied warnings as errors beside the others (DIA-07, CLI-27).
    fn warnings(&self, diagnostics: &Diagnostics, project: &Project) -> Result<Warnings, Failure> {
        let diagnostics = self.warning_levels.apply(diagnostics);
        if diagnostics.is_empty() {
            return Ok(Warnings::default());
        }
        if diagnostics.has_errors() {
            return Err(with_project(diagnostics, project));
        }
        Ok(Warnings {
            diagnostics,
            sources: Sources::Project(Box::new(project.clone())),
        })
    }

    /// Returns the runtime archive installed beside the compiler for the request's target.
    fn runtime_archive(&self, request: &BuildRequest) -> Result<RuntimeArchive, Failure> {
        RuntimeArchive::beside(&self.compiler, request.target().triple, request.mode())
            .map_err(Failure::without_sources)
    }

    /// Loads the program with the settings of its root and of the command line, as a library
    /// when `library` is true or the command line asks for one (CLI-09).
    /// Loads the program whose main file is `main` for `target`, whose operating system
    /// `sys.platform` names (LIB-22).
    fn load(&self, main: &Utf8Path, library: bool, target: Target) -> Result<Project, Failure> {
        let unloaded = |failure: typy_source::LoadFailure| Failure {
            diagnostics: failure.diagnostics.clone(),
            sources: Sources::Unloaded(Box::new(failure)),
        };
        let mut overrides = self.overrides;
        overrides.library |= library;
        let mut settings = ProjectSettings::read(&root_of(main), overrides).map_err(unloaded)?;
        settings.platform = if target.triple.is_macos() {
            Platform::MacOs
        } else {
            Platform::Linux
        };
        Project::load(main, settings).map_err(unloaded)
    }
}

/// Returns the syntax tree of the main module as text.
fn syntax_text(project: &Project) -> Emission {
    let module = project.main_module();
    let text = project
        .file(module.file)
        .map_or_else(String::new, |file| typy_source::dump(&module.syntax, file));
    Emission::Text(text)
}

fn with_project(diagnostics: typy_diagnostics::Diagnostics, project: &Project) -> Failure {
    Failure {
        diagnostics,
        sources: Sources::Project(Box::new(project.clone())),
    }
}

/// Returns a program's root: the main file's directory.
fn root_of(main: &Utf8Path) -> Utf8PathBuf {
    match main.parent() {
        Some(parent) if !parent.as_str().is_empty() => parent.to_owned(),
        _ => Utf8PathBuf::from("."),
    }
}

/// Writes the C header of `library` beside the library `output`: `NAME.h` for the module
/// `NAME` (FFI-05).
fn write_header(library: &TyirLibrary, output: &Utf8Path) -> Result<(), Diagnostics> {
    let path = output.with_file_name(format!("{}.h", library.name));
    let text = typy_codegen::c_header(library);
    std::fs::write(&path, text).map_err(|error| cannot_write(&path, &error))
}

/// Writes the binding module `NAME_binding.py`, which loads the shared library `soname`, and
/// its stub `NAME_binding.pyi`, beside the library `output` (FFI-08, ADR-0046).
fn write_binding(
    library: &TyirLibrary,
    output: &Utf8Path,
    soname: &str,
) -> Result<(), Diagnostics> {
    let name = &library.name;
    let files = [
        (
            format!("{name}_binding.py"),
            typy_codegen::python_binding(library, soname),
        ),
        (
            format!("{name}_binding.pyi"),
            typy_codegen::python_stub(library),
        ),
    ];
    for (file_name, text) in files {
        let path = output.with_file_name(file_name);
        std::fs::write(&path, text).map_err(|error| cannot_write(&path, &error))?;
    }
    Ok(())
}

/// Returns the E0106 of a file the build writes beside the library.
fn cannot_write(path: &Utf8Path, error: &std::io::Error) -> Diagnostics {
    Diagnostics::from(
        typy_diagnostics::Diagnostic::without_location(
            typy_diagnostics::DiagnosticCode::error(106),
            format!("cannot write {path}: {error}"),
        )
        .with_help("check that the directory exists and is writable"),
    )
}

/// Returns the linker of the request's target, with the allocator's library, each library
/// the program's `ctypes` declarations name, linked as `typy.toml` says, and the
/// directories it names (MEM-08, FFI-01, FFI-04).
fn linker(
    request: &BuildRequest,
    archive: RuntimeArchive,
    project: &Project,
    module: &TyirModule,
) -> Linker {
    // A release build leaves out the debug information, which a debug build keeps for a
    // debugger (CLI-05).
    let release = !request.mode().is_debug();
    let settings = project.settings();
    let triple = request.target().triple;
    let paths: Vec<Utf8PathBuf> = settings
        .library_paths
        .iter()
        .map(|path| settings.root.join(path))
        .collect();
    let mut libraries: Vec<String> = settings
        .allocator
        .iter()
        .filter_map(|allocator| allocator.library.as_ref())
        .map(|library| format!("-l{library}"))
        .collect();
    // `CDLL(None)` names the C and maths libraries, which every link has (FFI-14).
    for library in module.foreign_libraries.iter().flatten() {
        let link = settings
            .libraries
            .get(library)
            .copied()
            .unwrap_or(typy_source::LibraryLink::Dynamic);
        let statically = link == typy_source::LibraryLink::Static;
        libraries.push(typy_build::library_argument(
            triple, library, statically, &paths,
        ));
    }
    let mut linker = Linker::new(triple, archive)
        .with_strip_debug(release)
        .with_libraries(libraries)
        .with_library_paths(paths);
    if let Some(tools) = settings.targets.get(request.target().triple.as_str()) {
        linker = linker.with_tools(
            tools.linker.as_deref(),
            tools.archiver.as_deref(),
            tools.object_copier.as_deref(),
        );
    }
    linker
}

/// Returns the program in TyIR, allocating with the functions the project names (MEM-08).
/// Writes the object file of a build beside its output, named after the output's name up to
/// its first dot: `app.o` for `app`, `libtools.o` for `libtools.so.1` (CLI-24).
fn keep_object(bytes: &[u8], output: &Utf8Path) -> Result<(), Diagnostics> {
    let name = output.file_name().unwrap_or("output");
    let stem = name
        .split('.')
        .next()
        .filter(|stem| !stem.is_empty())
        .unwrap_or(name);
    let path = output.with_file_name(format!("{stem}.o"));
    std::fs::write(&path, bytes).map_err(|error| {
        Diagnostics::from(
            typy_diagnostics::Diagnostic::without_location(
                typy_diagnostics::DiagnosticCode::error(106),
                format!("cannot write {path}: {error}"),
            )
            .with_help("check that the directory exists and is writable"),
        )
    })
}

fn lower(program: &TypedProgram, project: &Project) -> Result<TyirModule, Failure> {
    let mut module =
        typy_lower::lower(program).map_err(|diagnostics| with_project(diagnostics, project))?;
    if let Some(allocator) = &project.settings().allocator {
        module.allocator = TyirAllocator {
            allocate: allocator.allocate.clone(),
            reallocate: allocator.reallocate.clone(),
            free: allocator.free.clone(),
        };
    }
    Ok(module)
}
