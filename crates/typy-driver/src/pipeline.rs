//! Runs the stages of one build in order and stops after the stage `--emit` names.

use camino::{Utf8Path, Utf8PathBuf};
use typy_build::{Artifact, BuildRequest, EmitStage, Linker, RuntimeArchive};
use typy_check::{TypeChecker, TypedProgram};
use typy_codegen::{assembly, llvm_ir, object};
use typy_diagnostics::Diagnostics;
use typy_source::{Project, ProjectSettings, SETTINGS_FILE, SettingOverrides};
use typy_tyir::TyirModule;

use crate::emission::Emission;
use crate::failure::Failure;
use crate::outcome::Outcome;
use crate::sources::Sources;
use crate::success::{Success, Warnings};

/// Runs builds and checks. It holds the compiler's location, which finds the runtime
/// archive, and the settings the command line gives, so one pipeline runs any number of
/// builds (detailed design section 12).
#[derive(Clone, Debug)]
pub struct Pipeline {
    compiler: Utf8PathBuf,
    overrides: SettingOverrides,
    deny_warnings: bool,
}

impl Pipeline {
    /// Returns a pipeline for the compiler executable at `compiler`, whose runtime archives are
    /// in `<prefix>/lib/typy/` beside `<prefix>/bin/`.
    #[must_use]
    pub fn new(compiler: impl Into<Utf8PathBuf>) -> Self {
        Self {
            compiler: compiler.into(),
            overrides: SettingOverrides::default(),
            deny_warnings: false,
        }
    }

    /// Returns the pipeline with warnings that fail a build or a check, before any code is
    /// generated (DIA-07).
    #[must_use]
    pub const fn with_deny_warnings(mut self, deny: bool) -> Self {
        self.deny_warnings = deny;
        self
    }

    /// Returns the pipeline with settings that win over each project's `typy.toml` (CLI-07).
    #[must_use]
    pub const fn with_overrides(mut self, overrides: SettingOverrides) -> Self {
        self.overrides = overrides;
        self
    }

    /// Loads and checks the program whose main file is `main`, without generating code (CLI-03).
    pub fn check(&self, main: &Utf8Path) -> Result<Success, Failure> {
        let project = self.load(main)?;
        let (_, warnings) = self.check_program(&project)?;
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
        let project = self.load(main).ok()?;
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
        let project = self.load(request.main())?;
        if stage == EmitStage::Ast {
            let warnings = self.warnings(project.warnings().clone(), &project)?;
            return Ok((syntax_text(&project), warnings));
        }
        let (program, warnings) = self.check_program(&project)?;
        if stage == EmitStage::TypedAst {
            let text = typy_check::dump(&program, &project);
            return Ok((Emission::Text(text), warnings));
        }
        let module = lower(&program, &project)?;
        let target = request.target();
        let optimization = request.mode().optimization();
        let failed = |diagnostics| with_project(diagnostics, &project);
        let emission = match stage {
            EmitStage::Ast => syntax_text(&project),
            EmitStage::TypedAst => Emission::Text(typy_check::dump(&program, &project)),
            EmitStage::Tyir => Emission::Text(typy_tyir::print_without_counting(&module)),
            EmitStage::TyirMem => Emission::Text(typy_tyir::print(&module)),
            EmitStage::LlvmIr => {
                Emission::Text(llvm_ir(&module, target, optimization, &project).map_err(failed)?)
            }
            EmitStage::Asm => {
                Emission::Text(assembly(&module, target, optimization, &project).map_err(failed)?)
            }
            EmitStage::Obj => Emission::Bytes(
                object(&module, target, optimization, &project)
                    .map_err(failed)?
                    .bytes,
            ),
        };
        Ok((emission, warnings))
    }

    /// Builds and links the executable. The runtime archive is opened first, so a broken
    /// installation fails before any work.
    fn build(&self, request: &BuildRequest) -> Result<(Artifact, Warnings), Failure> {
        let linker = self.linker(request)?;
        let project = self.load(request.main())?;
        let (program, warnings) = self.check_program(&project)?;
        let module = lower(&program, &project)?;
        let failed = |diagnostics| with_project(diagnostics, &project);
        let object = object(
            &module,
            request.target(),
            request.mode().optimization(),
            &project,
        )
        .map_err(failed)?;
        let artifact = linker
            .link(&object, &request.executable_path())
            .map_err(failed)?;
        Ok((artifact, warnings))
    }

    /// Checks the program, and returns it with every warning of loading and checking it.
    fn check_program(&self, project: &Project) -> Result<(TypedProgram, Warnings), Failure> {
        let program = TypeChecker::check(project)
            .map_err(|diagnostics| with_project(diagnostics, project))?;
        let mut found = project.warnings().clone();
        found.extend(program.warnings.clone());
        let warnings = self.warnings(found, project)?;
        Ok((program, warnings))
    }

    /// Returns the warnings, or a failure when there are some and warnings are denied
    /// (DIA-07).
    fn warnings(&self, diagnostics: Diagnostics, project: &Project) -> Result<Warnings, Failure> {
        if diagnostics.iter().next().is_none() {
            return Ok(Warnings::default());
        }
        if self.deny_warnings {
            return Err(with_project(diagnostics, project));
        }
        Ok(Warnings {
            diagnostics,
            sources: Sources::Project(Box::new(project.clone())),
        })
    }

    fn linker(&self, request: &BuildRequest) -> Result<Linker, Failure> {
        let triple = request.target().triple;
        let archive =
            RuntimeArchive::beside(&self.compiler, triple).map_err(Failure::without_sources)?;
        // A release build leaves out the debug information, which a debug build keeps for a
        // debugger (CLI-05).
        let release = request.mode() != typy_build::BuildMode::Debug;
        Ok(Linker::new(triple, archive).with_strip_debug(release))
    }

    /// Loads the program with the settings of its root and of the command line.
    fn load(&self, main: &Utf8Path) -> Result<Project, Failure> {
        let unloaded = |failure: typy_source::LoadFailure| Failure {
            diagnostics: failure.diagnostics.clone(),
            sources: Sources::Unloaded(Box::new(failure)),
        };
        let settings = ProjectSettings::read(&root_of(main), self.overrides).map_err(unloaded)?;
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

fn lower(program: &TypedProgram, project: &Project) -> Result<TyirModule, Failure> {
    typy_lower::lower(program).map_err(|diagnostics| with_project(diagnostics, project))
}
