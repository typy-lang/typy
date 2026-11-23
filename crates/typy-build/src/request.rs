//! What one build asks for: the main module, the mode, the target, the stage to stop at and
//! the output path.

use std::fmt;

use camino::{Utf8Path, Utf8PathBuf};
use typy_codegen::{CodegenOptions, Target};
use typy_source::MainSource;

use crate::emit_stage::EmitStage;
use crate::library_kind::LibraryKind;
use crate::mode::BuildMode;

/// Why a build request cannot be made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestError {
    /// The main file does not end in `.py`.
    NotPython(Utf8PathBuf),
}

impl fmt::Display for RequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPython(path) => write!(formatter, "{path} is not a `.py` file"),
        }
    }
}

impl std::error::Error for RequestError {}

/// What one build asks for. The main file's existence is checked when it is loaded (E0105).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildRequest {
    main: MainSource,
    mode: BuildMode,
    target: Target,
    emit: Option<EmitStage>,
    output: Option<Utf8PathBuf>,
    library: Option<LibraryKind>,
    /// The processor code is generated for; `None` for the target's baseline (CLI-21).
    processor: Option<String>,
    /// The features beside the processor's, as LLVM writes them (CLI-30).
    features: String,
}

impl BuildRequest {
    /// Returns a debug build of the main file `main` for the host, writing an executable.
    pub fn new(main: impl Into<Utf8PathBuf>) -> Result<Self, RequestError> {
        Self::of_main(MainSource::File(main.into()))
    }

    /// Returns a debug build for the host of the program whose main module `main` gives,
    /// writing an executable (CLI-28).
    pub fn of_main(main: MainSource) -> Result<Self, RequestError> {
        if let MainSource::File(path) = &main
            && path.extension() != Some("py")
        {
            return Err(RequestError::NotPython(path.clone()));
        }
        Ok(Self {
            main,
            mode: BuildMode::DEBUG,
            target: Target::host(),
            emit: None,
            output: None,
            library: None,
            processor: None,
            features: String::new(),
        })
    }

    /// Sets the build mode.
    #[must_use]
    pub const fn with_mode(mut self, mode: BuildMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the target.
    #[must_use]
    pub const fn with_target(mut self, target: Target) -> Self {
        self.target = target;
        self
    }

    /// Stops the build after `stage` and writes its output instead of an executable.
    #[must_use]
    pub const fn with_emit(mut self, stage: EmitStage) -> Self {
        self.emit = Some(stage);
        self
    }

    /// Sets the output path.
    #[must_use]
    pub fn with_output(mut self, output: impl Into<Utf8PathBuf>) -> Self {
        self.output = Some(output.into());
        self
    }

    /// Builds the main file as a C library of `kind` instead of an executable (CLI-09).
    #[must_use]
    pub const fn with_library(mut self, kind: LibraryKind) -> Self {
        self.library = Some(kind);
        self
    }

    /// Generates code for `processor`, or the target's baseline for `None`, with `features`
    /// turned on or off beside the processor's (CLI-21, CLI-30).
    #[must_use]
    pub fn with_processor(mut self, processor: Option<String>, features: String) -> Self {
        self.processor = processor;
        self.features = features;
        self
    }

    /// Returns what the build asks of code generation: the mode's optimisation, the processor
    /// and its features.
    #[must_use]
    pub fn codegen_options(&self) -> CodegenOptions {
        CodegenOptions {
            optimization: self.mode.optimization(),
            processor: self.processor.clone(),
            features: self.features.clone(),
        }
    }

    /// Returns the kind of C library the build writes, or `None` for an executable.
    #[must_use]
    pub const fn library(&self) -> Option<LibraryKind> {
        self.library
    }

    /// Returns where the main module comes from.
    #[must_use]
    pub const fn main(&self) -> &MainSource {
        &self.main
    }

    /// Returns the main file's directory, where a library goes by default; empty for a main
    /// module that is not a file.
    #[must_use]
    pub fn directory(&self) -> &Utf8Path {
        match &self.main {
            MainSource::File(path) => path.parent().unwrap_or(Utf8Path::new("")),
            MainSource::Module(_) | MainSource::Code(_) => Utf8Path::new(""),
        }
    }

    /// Returns the build mode.
    #[must_use]
    pub const fn mode(&self) -> BuildMode {
        self.mode
    }

    /// Returns the target.
    #[must_use]
    pub const fn target(&self) -> Target {
        self.target
    }

    /// Returns the stage the build stops after, if any.
    #[must_use]
    pub const fn emit(&self) -> Option<EmitStage> {
        self.emit
    }

    /// Returns the output path given, if any.
    #[must_use]
    pub fn output(&self) -> Option<&Utf8Path> {
        self.output.as_deref()
    }

    /// Returns where the executable goes: the output path, or the main file without `.py`,
    /// or the program's name for a main module that is not a file. A library's files are
    /// named by `LibraryLayout`, after its module's version.
    #[must_use]
    pub fn output_path(&self) -> Utf8PathBuf {
        self.output.clone().unwrap_or_else(|| match &self.main {
            MainSource::File(path) => path.with_extension(""),
            MainSource::Module(_) | MainSource::Code(_) => {
                Utf8PathBuf::from(self.main.program_name())
            }
        })
    }
}

#[cfg(test)]
mod tests;
