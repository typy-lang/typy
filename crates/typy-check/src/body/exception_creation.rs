//! What a check records of the exceptions a program creates, and the type of `args` the
//! checks before it found (EXC-15, ADR-0034).

use typy_diagnostics::Span;

use super::Program;
use crate::exception::ExceptionClass;
use crate::exception_arguments::Creations;
use crate::types::Type;

impl Program {
    /// Gives this check the creations the checks before it recorded, or none for a first
    /// check.
    pub(crate) fn set_known_creations(&mut self, known: Option<Creations>) {
        self.known_creations = known.map(std::rc::Rc::new);
    }

    /// Returns the creations of exceptions this check recorded.
    pub(crate) const fn creations(&self) -> &Creations {
        &self.creations
    }

    /// Returns where a body first reads `args` as a value, if one does.
    pub(crate) const fn first_arguments_read(&self) -> Option<Span> {
        self.arguments_read
    }

    /// Records a call or a raise of `class` with arguments of `arguments`.
    pub(super) fn record_creation(&mut self, class: &ExceptionClass, arguments: &[Type]) {
        self.creations.make(class, arguments);
    }

    /// Records `super().__init__(arguments)` in the `__init__` of `class`.
    pub(super) fn record_initialization(&mut self, class: &ExceptionClass, arguments: &[Type]) {
        self.creations.initialize(class, arguments);
    }

    /// Records a key the runtime looks up, whose `KeyError` holds it when it is missing.
    pub(super) fn record_key(&mut self, key: &Type) {
        self.creations.key(key);
    }

    /// Returns the type of `args` of an exception of `class` read at `span`, or an error in a
    /// first check, which knows no creation yet and is followed by another.
    pub(super) fn arguments_type(&mut self, class: &ExceptionClass, span: Span) -> Type {
        self.arguments_read.get_or_insert(span);
        self.known_creations
            .as_ref()
            .map_or(Type::Error, |known| known.arguments_of(class))
    }

    /// Returns the type of `args` of an exception of `class`, when a check before this one
    /// recorded the creations that give it.
    pub(super) fn known_arguments_type(&self, class: &ExceptionClass) -> Option<Type> {
        self.known_creations
            .as_ref()
            .map(|known| known.arguments_of(class))
    }
}
