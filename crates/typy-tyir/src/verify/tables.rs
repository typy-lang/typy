//! Checks the method tables of a module's classes: a class's table starts as its base's does,
//! and the entries at one position can be called the same way (ADR-0021).

use crate::function::FunctionId;
use crate::module::TyirModule;
use crate::types::{OptionalBase, TyirType};

use super::VerifyError;

/// Checks every class's table against its base's.
pub(super) fn check(module: &TyirModule) -> Result<(), VerifyError> {
    for (index, class) in module.classes.iter().enumerate() {
        let error = |message: String| VerifyError {
            function: format!("class#{index} {}", class.name),
            block: None,
            message,
        };
        for (slot, entry) in class.methods.iter().enumerate() {
            let Some(function) = module.functions.get(entry.0 as usize) else {
                return Err(error(format!(
                    "method#{slot} is fn{}, which does not exist",
                    entry.0
                )));
            };
            if function.parameters.is_empty() {
                return Err(error(format!("method#{slot} takes no instance")));
            }
        }
        let Some(base) = class.base else {
            continue;
        };
        let Some(inherited) = module.classes.get(base as usize) else {
            return Err(error(format!("the base class#{base} does not exist")));
        };
        // A base may come after the class, as a copy of a generic class does (ADR-0040), but
        // the chain of bases ends.
        if comes_back(module, index) {
            return Err(error(format!("the bases of class#{index} come back to it")));
        }
        if class.methods.len() < inherited.methods.len() {
            return Err(error(format!(
                "the table has {} methods, fewer than the {} of its base",
                class.methods.len(),
                inherited.methods.len()
            )));
        }
        for (slot, (own, base)) in class.methods.iter().zip(&inherited.methods).enumerate() {
            if !same_convention(module, *own, *base) {
                let (own, base) = (module.function(*own), module.function(*base));
                return Err(error(format!(
                    "method#{slot}, {}, is not called as {}, its base's entry, is",
                    own.symbol, base.symbol
                )));
            }
        }
    }
    Ok(())
}

/// Returns true when the chain of bases from the class `index` reaches the class again.
fn comes_back(module: &TyirModule, index: usize) -> bool {
    let mut current = module.classes[index].base;
    for _ in 0..module.classes.len() {
        match current {
            Some(each) if each as usize == index => return true,
            Some(each) => {
                current = module
                    .classes
                    .get(each as usize)
                    .and_then(|class| class.base);
            }
            None => return false,
        }
    }
    true
}

/// Returns true when two functions take the same arguments and give the same result in the
/// same registers: equal types, or two instances, optional or not, which are both addresses.
fn same_convention(module: &TyirModule, own: FunctionId, base: FunctionId) -> bool {
    let (own, base) = (module.function(own), module.function(base));
    let types = |function: &crate::function::TyirFunction| -> Vec<TyirType> {
        function
            .parameters
            .iter()
            .map(|value| function.value(*value).ty)
            .chain([function.return_type])
            .collect()
    };
    let (own_types, base_types) = (types(own), types(base));
    own.may_raise == base.may_raise
        && own_types.len() == base_types.len()
        && own_types
            .iter()
            .zip(&base_types)
            .all(|(own, base)| own == base || (is_address(*own) && is_address(*base)))
}

/// Returns true for an instance or an optional instance, held as an address or null.
fn is_address(ty: TyirType) -> bool {
    matches!(
        ty,
        TyirType::Object(id) | TyirType::Optional(OptionalBase::Object(id)) if id.class_index().is_some()
    )
}
