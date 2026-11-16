//! The copies of generic methods that override one another: for each copy of a generic method
//! that a class derived from its class defines again, the override's copy for the same type
//! arguments, both held in their classes' method tables under one key, so a call through the
//! base runs the object's class's copy (GEN-09).

use super::Program;
use crate::typed::{ClassId, FunctionId};
use crate::types::Type;

impl Program {
    /// Makes the copies of the overrides of every copy of a generic method made so far, and
    /// returns true when it made one, which a check of its body may follow with more.
    pub(crate) fn make_override_copies(&mut self) -> bool {
        let mut made_any = false;
        let copies = self.generics.copies.clone();
        for copy in copies {
            // A copy for a type parameter is checked, never run; a copy of a method of a copy
            // of a generic class is never overridden here.
            if copy.class.is_some() || copy.arguments.iter().any(Type::has_parameter) {
                continue;
            }
            let Some((holder, name)) = self.method_holder(copy.generic) else {
                continue;
            };
            let below = self.overriding_methods(holder, &name);
            if below.is_empty() {
                continue;
            }
            let given: Vec<String> = copy.arguments.iter().map(ToString::to_string).collect();
            let key = format!("{name}[{}]", given.join(", "));
            self.hold_copy(holder, &key, copy.copy);
            for (class, generic) in below {
                let known = self
                    .generics
                    .copies
                    .iter()
                    .any(|each| each.generic == generic && each.arguments == copy.arguments);
                let made = self.made_copy(generic, copy.arguments.clone(), None);
                made_any |= !known;
                self.hold_copy(class, &key, made);
            }
        }
        made_any
    }

    /// Returns the class whose methods hold the generic method `function`, with its name.
    fn method_holder(&self, function: FunctionId) -> Option<(ClassId, String)> {
        self.classes.iter().find_map(|info| {
            info.methods
                .iter()
                .find(|(_, each)| *each == function)
                .map(|(name, _)| (info.id, name.clone()))
        })
    }

    /// Returns each class derived from `class` that defines the generic method `name` again,
    /// with that method.
    fn overriding_methods(&self, class: ClassId, name: &str) -> Vec<(ClassId, FunctionId)> {
        self.classes
            .iter()
            .filter(|info| info.id != class && self.derives(info.id, class))
            .filter_map(|info| Some((info.id, info.method(name)?)))
            .filter(|(_, method)| self.generic(*method).is_some())
            .collect()
    }

    /// Returns true when `class` is `base` or derives from it.
    fn derives(&self, class: ClassId, base: ClassId) -> bool {
        std::iter::successors(Some(class), |each| self.classes[each.0 as usize].base)
            .any(|each| each == base)
    }

    /// Holds the copy `copy` in the method table of `class` under `key`, once.
    fn hold_copy(&mut self, class: ClassId, key: &str, copy: FunctionId) {
        let methods = &mut self.classes[class.0 as usize].methods;
        if !methods.iter().any(|(each, _)| each == key) {
            methods.push((key.to_owned(), copy));
        }
    }
}
