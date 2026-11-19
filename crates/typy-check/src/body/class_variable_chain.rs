//! Class variables a program assigns through a subclass that inherits them (CLS-25,
//! ADR-0037): the subclass holds a variable of its own, bound once assigned, and a read goes
//! from its class up to the first holder that is bound; a read through an instance goes
//! through a getter of the instance's class, found with the method table.

use super::Program;
use crate::check::subclass_variables::SubclassVariables;
use crate::typed::{
    ClassId, FunctionId, FunctionSignature, SignatureParameter, SymbolId, SymbolKind, TypedExpr,
    TypedExprKind, TypedFunction, TypedStmt,
};
use crate::types::Type;

/// Returns the name a class variable's getter is held under in the method table, which no
/// Python name can be.
pub(crate) fn getter_name(variable: &str) -> String {
    format!("{variable}.class")
}

impl Program {
    /// Notes the class variables the program assigns through a subclass.
    pub(crate) fn set_subclass_variables(&mut self, found: SubclassVariables) {
        self.subclass_variables = found;
    }

    /// Returns the variables the class `class` of the module being checked holds of its own,
    /// since the program assigns them through it while a base gives them.
    pub(crate) fn own_class_variables(&self, class: &str) -> Vec<String> {
        self.subclass_variables
            .own
            .iter()
            .filter(|(module, name, _)| *module == self.module && name == class)
            .map(|(_, _, variable)| variable.clone())
            .collect()
    }

    /// Returns the module and the name of a class.
    fn class_key(&self, class: ClassId) -> (String, String) {
        (
            self.class_modules[class.0 as usize].clone(),
            self.classes[class.0 as usize].name.clone(),
        )
    }

    /// Returns true when the program gives `class` a variable `name` of its own by an
    /// assignment through it.
    pub(super) fn holds_own(&self, class: ClassId, name: &str) -> bool {
        let (module, class) = self.class_key(class);
        self.subclass_variables
            .own
            .contains(&(module, class, name.to_owned()))
    }

    /// Returns true when some class derived from `holder` gets a variable `name` of its own,
    /// so a read through an instance reads the variable of the instance's class.
    pub(super) fn shares(&self, holder: ClassId, name: &str) -> bool {
        let (module, class) = self.class_key(holder);
        self.subclass_variables
            .shared
            .contains(&(module, class, name.to_owned()))
    }

    /// Returns the module variable that holds the class variable `name` of `class`.
    fn holder_symbol(&self, class: ClassId, name: &str) -> Option<SymbolId> {
        let (module, class_name) = self.class_key(class);
        let variable = crate::check::class_value_variable(&class_name, name);
        if module == self.module {
            return self.module_symbols.get(&variable).copied();
        }
        self.scopes
            .get(&module)?
            .module_symbols
            .get(&variable)
            .copied()
    }

    /// Returns the holders a read of `name` through `class` tests, from `class` up to
    /// `holder`, the class whose body gives it: each class that holds one of its own, then
    /// `holder`'s. Each holder of a subclass takes the type of `holder`'s.
    pub(super) fn variable_chain(
        &mut self,
        class: ClassId,
        holder: ClassId,
        name: &str,
    ) -> Vec<SymbolId> {
        let mut chain = Vec::new();
        let mut current = Some(class);
        while let Some(each) = current.filter(|each| *each != holder) {
            if self.holds_own(each, name) {
                chain.extend(self.holder_symbol(each, name));
            }
            current = self.classes[each.0 as usize].base;
        }
        let Some(last) = self.holder_symbol(holder, name) else {
            return chain;
        };
        let ty = self.symbols[last.0 as usize].ty.clone();
        for symbol in &chain {
            self.symbols[symbol.0 as usize].ty = ty.clone();
        }
        chain.push(last);
        chain
    }

    /// Returns the read of a class variable through `holders`: the module variable itself
    /// for one holder, and the first bound of several.
    pub(super) fn chain_read(
        &self,
        holders: Vec<SymbolId>,
        span: typy_diagnostics::Span,
    ) -> TypedExpr {
        let ty = holders
            .last()
            .map_or(Type::Error, |last| self.symbols[last.0 as usize].ty.clone());
        let kind = match holders.as_slice() {
            [only] => TypedExprKind::ModuleVariable(*only),
            _ => TypedExprKind::ClassVariable(holders),
        };
        TypedExpr { kind, ty, span }
    }

    /// Gives each class of the module being checked a getter for each class variable it
    /// reads that a class derived from the variable's class holds one of its own of, once the
    /// first look at the module body has given the variables their types.
    pub(crate) fn add_class_variable_getters(&mut self) {
        let classes: Vec<ClassId> = (0..self.classes.len())
            .filter(|index| self.class_modules[*index] == self.module)
            .map(|index| ClassId(u32::try_from(index).unwrap_or(u32::MAX)))
            .filter(|class| !self.class_copies.borrow().roots.contains_key(class))
            .collect();
        let shared: Vec<(String, String, String)> =
            self.subclass_variables.shared.iter().cloned().collect();
        for class in classes {
            for (module, holder_name, name) in &shared {
                let mut current = Some(class);
                while let Some(each) = current {
                    if self.class_key(each) == (module.clone(), holder_name.clone()) {
                        let getter = self.class_variable_getter(class, each, name);
                        self.classes[class.0 as usize]
                            .methods
                            .push((getter_name(name), getter));
                        break;
                    }
                    current = self.classes[each.0 as usize].base;
                }
            }
        }
    }

    /// Returns the getter of the class variable `name` that `holder` gives, for `class`.
    fn class_variable_getter(&mut self, class: ClassId, holder: ClassId, name: &str) -> FunctionId {
        let span = self.classes[class.0 as usize].span;
        let chain = self.variable_chain(class, holder, name);
        let read = self.chain_read(chain, span);
        let id = FunctionId(u32::try_from(self.functions.len()).unwrap_or(u32::MAX));
        let instance = self.instance_type(class);
        let this = self.add_symbol("self", SymbolKind::Parameter, instance.clone(), span);
        let signature = FunctionSignature {
            parameters: vec![SignatureParameter {
                name: "self".to_owned(),
                ty: instance,
                default: None,
                default_call: None,
                kind: typy_source::syntax::ParameterKind::Normal,
            }],
            returns: read.ty.clone(),
        };
        self.signatures.push(signature.clone());
        let class_name = self.classes[class.0 as usize].name.clone();
        self.functions.push(Some(TypedFunction {
            id,
            name: getter_name(name),
            qualified_name: format!("{class_name}.{}", getter_name(name)),
            module: self.class_modules[class.0 as usize].clone(),
            signature,
            parameters: vec![this],
            locals: Vec::new(),
            body: vec![TypedStmt::Return {
                value: Some(read),
                span,
            }],
            span,
            generic: false,
            named_targets: Vec::new(),
        }));
        id
    }
}
