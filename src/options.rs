//! Printer options (capabilities) returned by `GetAllOptions`.
//!
//! [`OptionsCollection`] provides an owned, framework-agnostic snapshot
//! of a printer's supported settings (copies, duplex, color mode, etc.).

/// A single printer option with its supported choices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionInfo {
    /// The option name, e.g. `"copies"` or `"sides"`.
    pub name: String,
    /// The default value as reported by the backend.
    pub default_value: String,
    /// The option group (e.g. `"General"`), or an empty string when unset.
    pub group: String,
    /// All values the printer supports for this option.
    pub supported_values: Vec<String>,
}

/// An owned snapshot of every option in a `cpdb_options_t`.
///
/// Built from D-Bus `GetAllOptions` responses (zbus backend) or from
/// C `cpdb_options_t` pointers (FFI backend). After construction, no raw
/// pointers are held - the collection is freely movable and cloneable.
///
/// # Example
///
/// ```rust
/// use cpdb_rs::options::{OptionsCollection, OptionInfo};
///
/// let col = OptionsCollection {
///     options: vec![OptionInfo {
///         name: "copies".to_string(),
///         default_value: "1".to_string(),
///         group: "General".to_string(),
///         supported_values: vec!["1".to_string(), "2".to_string()],
///     }],
/// };
/// assert_eq!(col.get("copies").unwrap().default_value, "1");
/// ```
#[derive(Debug, Clone, Default)]
pub struct OptionsCollection {
    /// Every option discovered, in iteration order of the underlying
    /// hash table (which itself is implementation-defined).
    pub options: Vec<OptionInfo>,
}

impl OptionsCollection {
    /// Builds an `OptionsCollection` from the D-Bus response tuples returned
    /// by [`crate::proxy::PrintBackendProxy::get_all_options()`].
    #[cfg(feature = "zbus-backend")]
    pub fn from_dbus(raw: Vec<crate::proxy::RawOption>) -> Self {
        let options = raw
            .into_iter()
            .map(|r| OptionInfo {
                name: r.option_name,
                group: r.group_name,
                default_value: r.default_value,
                supported_values: r.supported_values.into_iter().map(|(s,)| s).collect(),
            })
            .collect();
        Self { options }
    }

    /// Returns the number of options in this collection.
    #[inline]
    pub fn len(&self) -> usize {
        self.options.len()
    }

    /// Returns `true` if this collection has no options.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }

    /// Finds an option by name (linear search).
    pub fn get(&self, name: &str) -> Option<&OptionInfo> {
        self.options.iter().find(|o| o.name == name)
    }

    /// Returns an iterator over all options.
    pub fn iter(&self) -> impl Iterator<Item = &OptionInfo> {
        self.options.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_collection_helpers() {
        let col = OptionsCollection::default();
        assert!(col.is_empty());
        assert_eq!(col.len(), 0);
        assert!(col.get("copies").is_none());
    }

    #[test]
    fn collection_get_finds_by_name() {
        let col = OptionsCollection {
            options: vec![
                OptionInfo {
                    name: "copies".to_string(),
                    default_value: "1".to_string(),
                    group: "General".to_string(),
                    supported_values: vec!["1".to_string(), "2".to_string()],
                },
                OptionInfo {
                    name: "sides".to_string(),
                    default_value: "one-sided".to_string(),
                    group: "General".to_string(),
                    supported_values: vec![
                        "one-sided".to_string(),
                        "two-sided-long-edge".to_string(),
                    ],
                },
            ],
        };
        let found = col.get("sides");
        assert_eq!(found.unwrap().default_value, "one-sided");
        assert!(col.get("nonexistent").is_none());
    }

    #[test]
    fn collection_len_and_iter() {
        let col = OptionsCollection {
            options: vec![
                OptionInfo {
                    name: "a".into(),
                    default_value: String::new(),
                    group: String::new(),
                    supported_values: vec![],
                },
                OptionInfo {
                    name: "b".into(),
                    default_value: String::new(),
                    group: String::new(),
                    supported_values: vec![],
                },
            ],
        };
        assert_eq!(col.len(), 2);
        assert_eq!(col.iter().count(), 2);
    }
}
