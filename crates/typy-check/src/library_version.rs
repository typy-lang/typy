//! The version a library's file names carry: the release of its module's `__version__`, or
//! else of `version` in `typy.toml` (CLI-13, ADR-0045).

use typy_diagnostics::{Diagnostic, RuleId};
use typy_source::syntax::{Constant, Expr};
use typy_source::{Project, version_release};

use crate::exports::{module_bindings, refused};

/// Returns the release of the version of the library `project` builds, or `None` when its
/// module has no `__version__` and `typy.toml` no `version`.
pub(crate) fn library_version(project: &Project) -> Result<Option<String>, Box<Diagnostic>> {
    let module = project.main_module();
    let configured = project.settings().version.as_deref();
    let bindings = module_bindings(module, "__version__");
    let Some((value, span)) = bindings.first().copied() else {
        return Ok(configured.and_then(version_release).map(str::to_owned));
    };
    let wrong = |message: String, help: &str| {
        Box::new((*refused(span, message, help)).with_rule(RuleId::CLI_13))
    };
    if bindings.len() > 1 {
        return Err(wrong(
            "a library binds `__version__` once, and this binds it more than once".to_owned(),
            "keep the one assignment that gives the library's version",
        ));
    }
    let Some(Expr::Constant {
        value: Constant::Str(version),
        ..
    }) = value
    else {
        return Err(wrong(
            "a library's `__version__` is a string literal, whose release names its files"
                .to_owned(),
            "write it as `__version__ = \"5.0\"`, or remove it and set `version` in typy.toml",
        ));
    };
    let Some(release) = version_release(version) else {
        return Err(wrong(
            format!(
                "`__version__` is \"{version}\", which does not start with a release such as `5.0`"
            ),
            "start the version with its release numbers, such as `5.0rc1`",
        ));
    };
    if let Some(configured) = configured.filter(|configured| *configured != version) {
        return Err(wrong(
            format!(
                "`__version__` is \"{version}\" and `version` in typy.toml is \"{configured}\"; \
                 a library has one version"
            ),
            "remove `version` from typy.toml, or give both the same version",
        ));
    }
    Ok(Some(release.to_owned()))
}
