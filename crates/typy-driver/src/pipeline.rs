//! Runs the stages of one build in order and stops after the stage `--emit` names.

use camino::{Utf8Path, Utf8PathBuf};
use typy_build::{Artifact, BuildRequest, EmitStage, Linker, RuntimeArchive};
use typy_check::{TypeChecker, TypedProgram};
use typy_codegen::{assembly, llvm_ir, object};
use typy_source::{Project, ProjectSettings};
use typy_tyir::TyirModule;

use crate::emission::Emission;
use crate::failure::Failure;
use crate::outcome::Outcome;
use crate::sources::Sources;

/// Runs builds and checks. It holds only the compiler's location, which finds the runtime
/// archive, so one pipeline runs any number of builds (detailed design section 12).
#[derive(Clone, Debug)]
pub struct Pipeline {
    compiler: Utf8PathBuf,
}

impl Pipeline {
    /// Returns a pipeline for the compiler executable at `compiler`, whose runtime archives are
    /// in `<prefix>/lib/typy/` beside `<prefix>/bin/`.
    #[must_use]
    pub fn new(compiler: impl Into<Utf8PathBuf>) -> Self {
        Self {
            compiler: compiler.into(),
        }
    }

    /// Loads and checks the program whose main file is `main`, without generating code (CLI-03).
    pub fn check(&self, main: &Utf8Path) -> Result<Outcome, Failure> {
        let project = load(main)?;
        check(&project)?;
        Ok(Outcome::Checked(Sources::Project(Box::new(project))))
    }

    /// Runs the build `request` asks for: the output of one stage, or a linked executable.
    pub fn run(&self, request: &BuildRequest) -> Result<Outcome, Failure> {
        match request.emit() {
            Some(stage) => Self::emit(stage, request).map(Outcome::Emitted),
            None => self.build(request).map(Outcome::Executable),
        }
    }

    /// Runs the stages up to `stage` and returns its output; each stage runs only when the
    /// stages before it found no errors.
    fn emit(stage: EmitStage, request: &BuildRequest) -> Result<Emission, Failure> {
        let project = load(request.main())?;
        if stage == EmitStage::Ast {
            return Ok(syntax_text(&project));
        }
        let program = check(&project)?;
        if stage == EmitStage::TypedAst {
            return Ok(Emission::Text(typy_check::dump(&program, &project)));
        }
        let module = lower(&program, &project)?;
        let target = request.target();
        let optimization = request.mode().optimization();
        let failed = |diagnostics| with_project(diagnostics, &project);
        Ok(match stage {
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
        })
    }

    /// Builds and links the executable. The runtime archive is opened first, so a broken
    /// installation fails before any work.
    fn build(&self, request: &BuildRequest) -> Result<Artifact, Failure> {
        let linker = self.linker(request)?;
        let project = load(request.main())?;
        let program = check(&project)?;
        let module = lower(&program, &project)?;
        let failed = |diagnostics| with_project(diagnostics, &project);
        let object = object(
            &module,
            request.target(),
            request.mode().optimization(),
            &project,
        )
        .map_err(failed)?;
        linker
            .link(&object, &request.executable_path())
            .map_err(failed)
    }

    fn linker(&self, request: &BuildRequest) -> Result<Linker, Failure> {
        let triple = request.target().triple;
        let archive =
            RuntimeArchive::beside(&self.compiler, triple).map_err(Failure::without_sources)?;
        Ok(Linker::new(triple, archive))
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

/// Loads the program; its root is the main file's directory.
fn load(main: &Utf8Path) -> Result<Project, Failure> {
    let root = match main.parent() {
        Some(parent) if !parent.as_str().is_empty() => parent.to_owned(),
        _ => Utf8PathBuf::from("."),
    };
    Project::load(main, ProjectSettings::new(root)).map_err(|failure| Failure {
        diagnostics: failure.diagnostics.clone(),
        sources: Sources::Unloaded(Box::new(failure)),
    })
}

fn check(project: &Project) -> Result<TypedProgram, Failure> {
    TypeChecker::check(project).map_err(|diagnostics| with_project(diagnostics, project))
}

fn lower(program: &TypedProgram, project: &Project) -> Result<TyirModule, Failure> {
    typy_lower::lower(program).map_err(|diagnostics| with_project(diagnostics, project))
}
