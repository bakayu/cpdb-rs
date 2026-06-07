//! Printer options (capabilities) from C `cpdb_options_t`.

use crate::bindings as ffi;
use crate::error::{CpdbError, Result};
use crate::util;
use glib_sys::{GHashTableIter, g_hash_table_iter_init, g_hash_table_iter_next};
use std::mem::MaybeUninit;

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
#[derive(Debug, Clone, Default)]
pub struct OptionsCollection {
    /// Every option discovered.
    pub options: Vec<OptionInfo>,
}

impl OptionsCollection {
    /// Builds an [`OptionsCollection`] by iterating `raw.table`.
    ///
    /// # Safety
    ///
    /// `raw` must be either null or a valid pointer to a fully initialised
    /// `cpdb_options_t` whose `table` field is a valid `GHashTable*`.
    pub unsafe fn from_raw(raw: *mut ffi::cpdb_options_t) -> Result<Self> {
        if raw.is_null() {
            return Err(CpdbError::NullPointer);
        }

        let table = unsafe { (*raw).table };

        if table.is_null() {
            return Ok(Self::default());
        }

        let mut options: Vec<OptionInfo> = Vec::new();

        unsafe {
            let mut iter = MaybeUninit::<GHashTableIter>::uninit();
            g_hash_table_iter_init(iter.as_mut_ptr(), table as *mut glib_sys::GHashTable);
            let mut iter = iter.assume_init();

            let mut key: *mut libc::c_void = std::ptr::null_mut();
            let mut value: *mut libc::c_void = std::ptr::null_mut();

            while g_hash_table_iter_next(&mut iter, &mut key, &mut value) != 0 {
                if value.is_null() {
                    continue;
                }
                let opt = value as *mut ffi::cpdb_option_t;

                let name = util::cstr_to_string((*opt).option_name).unwrap_or_default();
                let default_value = util::cstr_to_string((*opt).default_value).unwrap_or_default();
                let group = util::cstr_to_string((*opt).group_name).unwrap_or_default();

                let mut supported_values: Vec<String> =
                    Vec::with_capacity((*opt).num_supported as usize);

                if !(*opt).supported_values.is_null() && (*opt).num_supported > 0 {
                    for i in 0..((*opt).num_supported as usize) {
                        let s_ptr = *(*opt).supported_values.add(i);
                        if !s_ptr.is_null() {
                            if let Ok(s) = util::cstr_to_string(s_ptr) {
                                supported_values.push(s);
                            }
                        }
                    }
                }

                options.push(OptionInfo {
                    name,
                    default_value,
                    group,
                    supported_values,
                });
            }
        }

        Ok(Self { options })
    }

    /// Returns the number of options in this collection.
    pub fn len(&self) -> usize {
        self.options.len()
    }

    /// Returns `true` if this collection has no options.
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
