//! What one build asks for: the main file, the mode, the target, the stage to stop at and the
//! output path.

use std::fmt;

use camino::{Utf8Path, Utf8PathBuf};
use typy_codegen::Target;

use crate::emit_stage::EmitStage;
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
    main: Utf8PathBuf,
    mode: BuildMode,
    target: Target,
    emit: Option<EmitStage>,
    output: Option<Utf8PathBuf>,
    deny_warnings: bool,
}

impl BuildRequest {
    /// Returns a debug build of `main` for the host, writing an executable.
    pub fn new(main: impl Into<Utf8PathBuf>) -> Result<Self, RequestError> {
        let main = main.into();
        if main.extension() != Some("py") {
            return Err(RequestError::NotPython(main));
        }
        Ok(Self {
            main,
            mode: BuildMode::Debug,
            target: Target::host(),
            emit: None,
            output: None,
            deny_warnings: false,
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

    /// Makes every warning fail the build (DIA-07).
    #[must_use]
    pub const fn with_deny_warnings(mut self, deny: bool) -> Self {
        self.deny_warnings = deny;
        self
    }

    /// Returns the main file.
    #[must_use]
    pub fn main(&self) -> &Utf8Path {
        &self.main
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

    /// Returns true when warnings fail the build.
    #[must_use]
    pub const fn deny_warnings(&self) -> bool {
        self.deny_warnings
    }

    /// Returns where the executable goes: the output path, or the main file without `.py`.
    #[must_use]
    pub fn executable_path(&self) -> Utf8PathBuf {
        self.output
            .clone()
            .unwrap_or_else(|| self.main.with_extension(""))
    }
}

#[cfg(test)]
mod tests {
    use super::{BuildRequest, RequestError};
    use crate::{BuildMode, EmitStage};
    use typy_codegen::{Target, TargetTriple};

    #[test]
    fn a_new_request_is_a_debug_build_of_an_executable_for_the_host() {
        let request = BuildRequest::new("src/app.py").unwrap();
        assert_eq!(request.main(), "src/app.py");
        assert_eq!(request.mode(), BuildMode::Debug);
        assert_eq!(request.target(), Target::host());
        assert_eq!((request.emit(), request.output()), (None, None));
        assert!(!request.deny_warnings());
        assert_eq!(request.executable_path(), "src/app");
    }

    #[test]
    fn every_setting_can_be_changed() {
        let musl = Target {
            triple: TargetTriple::X86_64LinuxMusl,
        };
        let request = BuildRequest::new("app.py")
            .unwrap()
            .with_mode(BuildMode::ReleaseSmall)
            .with_target(musl)
            .with_emit(EmitStage::Tyir)
            .with_output("out/app")
            .with_deny_warnings(true);
        assert_eq!(request.mode(), BuildMode::ReleaseSmall);
        assert_eq!(request.target(), musl);
        assert_eq!(request.emit(), Some(EmitStage::Tyir));
        assert_eq!(request.executable_path(), "out/app");
        assert!(request.deny_warnings());
    }

    #[test]
    fn a_main_file_that_is_not_python_is_refused() {
        for path in ["app", "app.ty", "app.py.txt", ""] {
            let error = BuildRequest::new(path).unwrap_err();
            assert_eq!(error, RequestError::NotPython(path.into()));
            assert_eq!(error.to_string(), format!("{path} is not a `.py` file"));
        }
    }
}
