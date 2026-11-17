//! Each export of a library calls a function of the module whose parameters and result are
//! the TyPy types of the export's C types (FFI-05).

use super::VerifyError;
use crate::library::CType;
use crate::module::TyirModule;
use crate::types::TyirType;

/// Returns the TyPy type an export's C type converts to and from; an export has the C
/// types of `int`, `float` and `bool` only (ADR-0032).
const fn tyir_type(ty: CType) -> TyirType {
    ty.result_type()
}

pub(super) fn check(module: &TyirModule) -> Result<(), VerifyError> {
    let Some(library) = &module.library else {
        return Ok(());
    };
    for export in &library.exports {
        let error = |message: String| VerifyError {
            function: format!("export {}", export.name),
            block: None,
            message,
        };
        let Some(function) = module.functions.get(export.function.0 as usize) else {
            return Err(error(format!("fn{} does not exist", export.function.0)));
        };
        let parameters: Vec<TyirType> = function
            .parameters
            .iter()
            .map(|value| function.value(*value).ty)
            .collect();
        let expected: Vec<TyirType> = export.parameters.iter().copied().map(tyir_type).collect();
        if parameters != expected {
            return Err(error(format!(
                "fn{} takes {parameters:?}, not the export's {expected:?}",
                export.function.0
            )));
        }
        let result = export.result.map_or(TyirType::Unit, tyir_type);
        if function.return_type != result {
            return Err(error(format!(
                "fn{} returns {}, not the export's {result}",
                export.function.0, function.return_type
            )));
        }
    }
    Ok(())
}
