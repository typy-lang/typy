//! Finds the classes of the program whose instances are iterators or iterables, as `typing`
//! reads the protocols (TYP-20, ADR-0065): a class whose `__next__` returns `T` and whose
//! `__iter__` returns the instance is an `Iterator[T]`, and a class whose `__iter__` returns an
//! `Iterator[T]`, a generator of `T` or an instance of such an iterator class is an
//! `Iterable[T]`.

use super::Program;
use crate::iterator_fits::{Iteration, note};
use crate::typed::{ClassId, FunctionId};
use crate::types::Type;

impl Program {
    /// Notes what the instances of each iterator or iterable class are, among `classes`, or
    /// among every class when `classes` is `None`. Iterators come first, so a class whose
    /// `__iter__` returns one of them is found an iterable.
    pub(crate) fn note_iterator_classes(&mut self, classes: Option<&[ClassId]>) {
        let candidates: Vec<ClassId> = match classes {
            Some(classes) => classes.to_vec(),
            None => self.classes.iter().map(|info| info.id).collect(),
        };
        let candidates: Vec<ClassId> = candidates
            .into_iter()
            .filter(|class| {
                !self.is_protocol(*class)
                    && !self.is_opaque_class(*class)
                    && self.exception(*class).is_none()
            })
            .collect();
        for pass in [true, false] {
            for class in &candidates {
                if self.protocol_values.iterations.contains_key(class) {
                    continue;
                }
                let found = if pass {
                    self.iterator_item(*class).map(Iteration::Iterator)
                } else {
                    self.iterable_item(*class).map(Iteration::Iterable)
                };
                if let Some(iteration) = found {
                    self.protocol_values
                        .iterations
                        .insert(*class, iteration.clone());
                    note(*class, iteration);
                }
            }
        }
    }

    /// Returns what `__next__` gives when `class` is an iterator: its `__iter__` returns an
    /// instance of the class or of a base of it, which is the instance itself.
    fn iterator_item(&self, class: ClassId) -> Option<Type> {
        let next = self.method_of_self(class, "__next__")?;
        let iter = self.method_of_self(class, "__iter__")?;
        let Type::Instance(returned) = &self.signatures[iter.0 as usize].returns else {
            return None;
        };
        let Type::Instance(own) = self.instance_type(class) else {
            return None;
        };
        own.derives_from(returned)
            .then(|| self.signatures[next.0 as usize].returns.clone())
            .filter(|item| !item.has_error())
    }

    /// Returns the type of each item of the iterator that `__iter__` of `class` returns.
    fn iterable_item(&self, class: ClassId) -> Option<Type> {
        let iter = self.method_of_self(class, "__iter__")?;
        match &self.signatures[iter.0 as usize].returns {
            Type::Iterator(item) => Some(item.as_ref().clone()),
            Type::Generator { yields, .. } => Some(yields.as_ref().clone()),
            Type::Instance(returned) => match self.protocol_values.iterations.get(&returned.id) {
                Some(Iteration::Iterator(item)) => Some(item.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Returns the method `name` of `class`, its own or a base's, when it takes the instance
    /// alone, as the runtime calls it.
    fn method_of_self(&self, class: ClassId, name: &str) -> Option<FunctionId> {
        let method = self.method(class, name)?;
        (self.signatures[method.0 as usize].parameters.len() == 1).then_some(method)
    }
}
