//! A class of the program as the type of its instances, with the chain of its bases (CLS-01,
//! CLS-04, ADR-0021).

use std::fmt;

use crate::typed::ClassId;

/// The class of an instance: its ID, its name for messages, and the classes of the program it
/// derives from, nearest first. Two classes with the same ID have the same chain, so equality
/// is that of the IDs.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InstanceClass {
    /// The class.
    pub id: ClassId,
    /// Its name.
    pub name: String,
    /// Its bases that are classes of the program, nearest first, each with its name.
    pub ancestors: Vec<(ClassId, String)>,
}

impl InstanceClass {
    /// Returns a class that derives from `object` alone.
    #[must_use]
    pub fn root(id: ClassId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            ancestors: Vec::new(),
        }
    }

    /// Returns true when an instance of this class is an instance of `base`: the class itself
    /// or one it derives from.
    #[must_use]
    pub fn derives_from(&self, base: &Self) -> bool {
        self.id == base.id || self.ancestors.iter().any(|(id, _)| *id == base.id)
    }

    /// Returns the class's base, when it is a class of the program.
    #[must_use]
    pub fn base(&self) -> Option<Self> {
        let ((id, name), rest) = self.ancestors.split_first()?;
        Some(Self {
            id: *id,
            name: name.clone(),
            ancestors: rest.to_vec(),
        })
    }

    /// Returns the nearest class both derive from, if any.
    #[must_use]
    pub fn common_base(&self, other: &Self) -> Option<Self> {
        let mut current = Some(self.clone());
        while let Some(class) = current {
            if other.derives_from(&class) {
                return Some(class);
            }
            current = class.base();
        }
        None
    }
}

impl fmt::Display for InstanceClass {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::InstanceClass;
    use crate::typed::ClassId;

    fn chain() -> (InstanceClass, InstanceClass, InstanceClass) {
        let shape = InstanceClass::root(ClassId(0), "Shape");
        let circle = InstanceClass {
            id: ClassId(1),
            name: "Circle".to_owned(),
            ancestors: vec![(ClassId(0), "Shape".to_owned())],
        };
        let disc = InstanceClass {
            id: ClassId(2),
            name: "Disc".to_owned(),
            ancestors: vec![
                (ClassId(1), "Circle".to_owned()),
                (ClassId(0), "Shape".to_owned()),
            ],
        };
        (shape, circle, disc)
    }

    #[test]
    fn a_class_derives_from_itself_and_each_ancestor_but_not_a_subclass() {
        let (shape, circle, disc) = chain();
        assert!(disc.derives_from(&disc));
        assert!(disc.derives_from(&circle));
        assert!(disc.derives_from(&shape));
        assert!(!shape.derives_from(&circle));
    }

    #[test]
    fn the_base_keeps_the_rest_of_the_chain() {
        let (shape, circle, disc) = chain();
        assert_eq!(disc.base(), Some(circle));
        assert_eq!(shape.base(), None);
    }

    #[test]
    fn the_common_base_is_the_nearest_shared_class() {
        let (shape, circle, disc) = chain();
        let square = InstanceClass {
            id: ClassId(3),
            name: "Square".to_owned(),
            ancestors: vec![(ClassId(0), "Shape".to_owned())],
        };
        assert_eq!(disc.common_base(&circle), Some(circle.clone()));
        assert_eq!(disc.common_base(&square), Some(shape));
        assert_eq!(
            square.common_base(&InstanceClass::root(ClassId(4), "Other")),
            None
        );
    }
}
