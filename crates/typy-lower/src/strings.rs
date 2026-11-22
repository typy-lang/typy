//! The module's string pool: each distinct string constant once.

use std::collections::BTreeMap;

use typy_tyir::StringId;

/// Collects string constants and gives each distinct one an ID.
#[derive(Debug, Default)]
pub(crate) struct StringPool {
    strings: Vec<String>,
    ids: BTreeMap<String, StringId>,
    /// The strings that hold a surrogate, as code points (LEX-10).
    points: Vec<Vec<u32>>,
    point_ids: BTreeMap<Vec<u32>, u32>,
}

impl StringPool {
    /// Returns the ID of `text`, adding it on first use.
    pub(crate) fn intern(&mut self, text: &str) -> StringId {
        if let Some(id) = self.ids.get(text) {
            return *id;
        }
        let id = StringId(u32::try_from(self.strings.len()).unwrap_or(u32::MAX));
        self.strings.push(text.to_owned());
        self.ids.insert(text.to_owned(), id);
        id
    }

    /// Returns the index of the string with the code points `points`, adding it on first use.
    pub(crate) fn intern_points(&mut self, points: &[u32]) -> u32 {
        if let Some(index) = self.point_ids.get(points) {
            return *index;
        }
        let index = u32::try_from(self.points.len()).unwrap_or(u32::MAX);
        self.points.push(points.to_vec());
        self.point_ids.insert(points.to_vec(), index);
        index
    }

    /// Returns the strings in ID order, and the strings that hold a surrogate in index order.
    pub(crate) fn into_tables(self) -> (Vec<String>, Vec<Vec<u32>>) {
        (self.strings, self.points)
    }
}

#[cfg(test)]
mod tests;
