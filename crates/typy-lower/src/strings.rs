//! The module's string pool: each distinct string constant once.

use std::collections::BTreeMap;

use typy_tyir::StringId;

/// Collects string constants and gives each distinct one an ID.
#[derive(Debug, Default)]
pub(crate) struct StringPool {
    strings: Vec<String>,
    ids: BTreeMap<String, StringId>,
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

    /// Returns the strings in ID order.
    pub(crate) fn into_strings(self) -> Vec<String> {
        self.strings
    }
}

#[cfg(test)]
mod tests {
    use super::StringPool;

    #[test]
    fn each_distinct_string_is_kept_once_in_order() {
        let mut pool = StringPool::default();
        let first = pool.intern("a");
        let second = pool.intern("b");
        assert_eq!(pool.intern("a"), first);
        assert_ne!(first, second);
        assert_eq!(pool.into_strings(), ["a", "b"]);
    }
}
