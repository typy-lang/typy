//! The copies of generic classes: one for each set of type arguments the program reaches,
//! each a class of its own whose attributes and methods are the generic class's with the type
//! parameters replaced (GEN-03, ADR-0028).

use std::collections::BTreeMap;

use typy_diagnostics::Span;

use super::Program;
use super::generic_copy::GenericCopy;
use crate::class_type::InstanceClass;
use crate::generic::TypeParameter;
use crate::typed::{ClassId, FunctionId};
use crate::types::Type;

/// A generic class of the program.
#[derive(Clone, Debug)]
pub(crate) struct GenericClass {
    pub(crate) parameters: Vec<TypeParameter>,
    /// The type of its own instances, inside its body, where its parameters are opaque.
    pub(crate) instance: InstanceClass,
}

/// A copy of a generic class for its type arguments.
#[derive(Clone, Debug)]
pub(crate) struct ClassCopy {
    pub(crate) root: ClassId,
    pub(crate) arguments: Vec<Type>,
    pub(crate) id: ClassId,
    /// The type of its instances.
    pub(crate) instance: Type,
    /// Whether its information has been added to the program.
    pub(crate) made: bool,
}

/// The generic classes and their copies.
#[derive(Clone, Debug, Default)]
pub(crate) struct ClassCopies {
    pub(crate) roots: BTreeMap<ClassId, GenericClass>,
    pub(crate) copies: Vec<ClassCopy>,
    /// The ID the next copy takes: one past every class known so far.
    pub(crate) next: usize,
}

impl ClassCopies {
    /// Returns the type of `root` applied to `arguments`: the generic class's own instances
    /// when the arguments are its own parameters, and otherwise the copy's, made when new.
    pub(crate) fn applied(&mut self, root: ClassId, arguments: Vec<Type>) -> Type {
        let Some(generic) = self.roots.get(&root) else {
            return Type::Error;
        };
        let own = arguments.len() == generic.parameters.len()
            && arguments.iter().zip(&generic.parameters).all(|(argument, parameter)| {
                matches!(argument, Type::Parameter(given) if **given == *parameter)
            });
        if own {
            return Type::instance(generic.instance.clone());
        }
        if let Some(known) = self
            .copies
            .iter()
            .find(|copy| copy.root == root && copy.arguments == arguments)
        {
            return known.instance.clone();
        }
        let id = ClassId(u32::try_from(self.next).unwrap_or(u32::MAX));
        self.next += 1;
        let given: Vec<String> = arguments.iter().map(ToString::to_string).collect();
        let name = format!("{}[{}]", generic.instance.name, given.join(", "));
        let operators = generic.instance.operators;
        let ancestors = self.copy_ancestors(root, &arguments);
        let instance = Type::instance(InstanceClass {
            id,
            name,
            ancestors,
            operators,
            generic: Some((root, arguments.clone())),
            protocol: false,
        });
        self.copies.push(ClassCopy {
            root,
            arguments,
            id,
            instance: instance.clone(),
            made: false,
        });
        instance
    }

    /// Returns the copy with ID `id`, if it is one.
    pub(crate) fn copy(&self, id: ClassId) -> Option<&ClassCopy> {
        self.copies.iter().find(|copy| copy.id == id)
    }

    /// Returns the classes a copy of `root` for `arguments` derives from: the generic class's,
    /// each copy among them made for the arguments in place of the generic class's own
    /// parameters (GEN-08).
    fn copy_ancestors(&mut self, root: ClassId, arguments: &[Type]) -> Vec<(ClassId, String)> {
        let Some(generic) = self.roots.get(&root) else {
            return Vec::new();
        };
        let ancestors = generic.instance.ancestors.clone();
        let Some(owner) = generic.parameters.first().map(|parameter| parameter.owner) else {
            return ancestors;
        };
        ancestors
            .into_iter()
            .map(|(id, name)| {
                let Some(copy) = self.copy(id).cloned() else {
                    return (id, name);
                };
                let replaced = copy
                    .arguments
                    .iter()
                    .map(|each| self.substitute(each, owner, arguments))
                    .collect();
                match self.applied(copy.root, replaced) {
                    Type::Instance(made) => (made.id, made.name.clone()),
                    _ => (id, name),
                }
            })
            .collect()
    }

    /// Returns `ty` with each type parameter of `owner` replaced by its argument, a generic
    /// class's own instances and each copy becoming the copy for the replaced arguments.
    pub(crate) fn substitute(&mut self, ty: &Type, owner: Span, arguments: &[Type]) -> Type {
        ty.substituted_with(owner, arguments, &mut |class: &InstanceClass| {
            let (root, given) = if let Some((root, given)) = &class.generic {
                (*root, given.clone())
            } else {
                let own = self
                    .roots
                    .get(&class.id)?
                    .parameters
                    .iter()
                    .cloned()
                    .map(Type::parameter);
                (class.id, own.collect())
            };
            let replaced = given
                .iter()
                .map(|each| self.substitute(each, owner, arguments))
                .collect();
            Some(self.applied(root, replaced))
        })
    }
}

impl Program {
    /// Returns `ty` with each type parameter of `owner` replaced by its argument, a generic
    /// class's own instances and each copy becoming the copy for the replaced arguments.
    pub(crate) fn substitute(&self, ty: &Type, owner: Span, arguments: &[Type]) -> Type {
        self.class_copies
            .borrow_mut()
            .substitute(ty, owner, arguments)
    }

    /// Adds to the program each copy of a generic class asked for and not yet added: the
    /// generic class's information with the type parameters replaced, and a copy of each of
    /// its methods, checked when the module's other copies are.
    pub(crate) fn make_class_copies(&mut self) {
        loop {
            let pending = {
                let copies = self.class_copies.borrow();
                let mut pending: Vec<ClassCopy> = copies
                    .copies
                    .iter()
                    .filter(|copy| !copy.made)
                    .cloned()
                    .collect();
                pending.sort_by_key(|copy| copy.id);
                pending
            };
            if pending.is_empty() {
                return;
            }
            for copy in pending {
                self.make_class_copy(&copy);
            }
        }
    }

    fn make_class_copy(&mut self, copy: &ClassCopy) {
        for each in &mut self.class_copies.borrow_mut().copies {
            if each.id == copy.id {
                each.made = true;
            }
        }
        let root = self.classes[copy.root.0 as usize].clone();
        let owner = self.class_copies.borrow().roots[&copy.root]
            .parameters
            .first()
            .map(|parameter| parameter.owner);
        let mut info = root.clone();
        info.id = copy.id;
        // A generic class has a parameter, so its owner is known.
        let owner = owner.unwrap_or(root.span);
        for attribute in &mut info.attributes {
            attribute.ty = attribute
                .ty
                .as_ref()
                .map(|ty| self.substitute(ty, owner, &copy.arguments));
        }
        let mut renamed: BTreeMap<FunctionId, FunctionId> = BTreeMap::new();
        // A static method that names no type parameter is not generic, and every copy calls
        // it as it is.
        for (_, method) in root.methods.iter().chain(&root.class_methods) {
            if self.generic(*method).is_some() {
                let made = self.made_copy(*method, copy.arguments.clone(), Some(copy.id));
                renamed.insert(*method, made);
            }
        }
        let rename = |id: &FunctionId| renamed.get(id).copied().unwrap_or(*id);
        info.methods = root
            .methods
            .iter()
            .map(|(name, id)| (name.clone(), rename(id)))
            .collect();
        info.class_methods = root
            .class_methods
            .iter()
            .map(|(name, id)| (name.clone(), rename(id)))
            .collect();
        info.init = root.init.as_ref().map(rename);
        info.adapted = root
            .adapted
            .iter()
            .map(|(name, base, own)| (name.clone(), rename(base), rename(own)))
            .collect();
        info.adapters = Vec::new();
        info.static_bridges = Vec::new();
        let module = self.class_modules[copy.root.0 as usize].clone();
        self.classes.push(info);
        self.class_modules.push(module);
        self.instances.insert(copy.id, copy.instance.clone());
        self.add_comparison_bridges(copy.id);
        self.add_truth_bridge(copy.id);
        self.add_override_adapters(copy.id);
        self.add_static_bridges(copy.id);
    }

    /// Returns true when the function is a copy whose type arguments hold a type parameter,
    /// as one made for a type an opaque body names is: it is checked, never run (ADR-0023).
    pub(crate) fn is_opaque_copy(&self, function: FunctionId) -> bool {
        self.generics.copies.iter().any(|copy: &GenericCopy| {
            copy.copy == function && copy.arguments.iter().any(Type::has_parameter)
        })
    }

    /// Returns true when the class is a protocol, a generic class, or a copy whose type
    /// arguments hold a type parameter: it has no instances and is never laid out (GEN-04,
    /// ADR-0028).
    pub(crate) fn is_opaque_class(&self, class: ClassId) -> bool {
        let copies = self.class_copies.borrow();
        self.is_protocol(class)
            || copies.roots.contains_key(&class)
            || copies
                .copy(class)
                .is_some_and(|copy| copy.arguments.iter().any(Type::has_parameter))
    }

    /// Returns the type of attribute `index` of a copy of a generic class, from the generic
    /// class's, with the copy's type arguments; `None` for any other class, or while the
    /// generic class's own type is not known.
    pub(super) fn copied_attribute_ty(&self, class: ClassId, index: usize) -> Option<Type> {
        let (root, arguments) = {
            let copies = self.class_copies.borrow();
            let copy = copies.copy(class)?;
            (copy.root, copy.arguments.clone())
        };
        let owner = self
            .class_copies
            .borrow()
            .roots
            .get(&root)?
            .parameters
            .first()?
            .owner;
        let ty = self.classes[root.0 as usize]
            .attributes
            .get(index)?
            .ty
            .clone()?;
        Some(self.substitute(&ty, owner, &arguments))
    }

    /// Returns the name of a class, as its `class` statement writes it.
    pub(crate) fn class_name(&self, class: ClassId) -> &str {
        &self.classes[class.0 as usize].name
    }
}
