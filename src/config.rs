//! Printer configuration persistence types.
//!
//! Provides a helper structure to store global defaults and per-printer overrides.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Saved user preferences for the print dialog.
///
/// Keeps track of the last selected printer, global default settings,
/// and per-printer customized settings (e.g. paper size, quality options).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PrinterConfig {
    /// Last-used printer identifier
    pub last_printer_id: Option<String>,
    /// Last-used backend name
    pub last_backend: Option<String>,
    /// Global settings that apply to all printers unless overridden (e.g. `"copies" => "1"`)
    pub global_settings: HashMap<String, String>,
    /// Per-printer settings keyed by `"printer_id@backend"`
    pub printer_settings: HashMap<String, HashMap<String, String>>,
}

impl PrinterConfig {
    /// Creates a new empty `PrinterConfig`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if this config has a saved printer.
    pub fn has_printer(&self) -> bool {
        self.last_printer_id.is_some()
    }

    /// Sets the last-used printer and backend.
    pub fn set_last_printer(&mut self, printer_id: &str, backend: &str) {
        self.last_printer_id = Some(printer_id.to_string());
        self.last_backend = Some(backend.to_string());
    }

    /// Gets a global setting.
    pub fn get_global_setting(&self, key: &str) -> Option<&str> {
        self.global_settings.get(key).map(|s| s.as_str())
    }

    /// Sets a global setting.
    pub fn set_global_setting(&mut self, key: String, value: String) {
        self.global_settings.insert(key, value);
    }

    /// Gets the merged settings for a specific printer.
    ///
    /// This combines the `global_settings` with the printer-specific settings,
    /// where printer-specific values override global values.
    pub fn get_settings_for(&self, printer_id: &str, backend: &str) -> HashMap<String, String> {
        let mut merged = self.global_settings.clone();
        let key = format!("{}@{}", printer_id, backend);
        if let Some(specific) = self.printer_settings.get(&key) {
            for (k, v) in specific {
                merged.insert(k.clone(), v.clone());
            }
        }
        merged
    }

    /// Updates or replaces the settings for a specific printer.
    pub fn set_settings_for(
        &mut self,
        printer_id: &str,
        backend: &str,
        settings: HashMap<String, String>,
    ) {
        let key = format!("{}@{}", printer_id, backend);
        self.printer_settings.insert(key, settings);
    }

    /// Sets or updates a single setting for a specific printer.
    pub fn set_setting_for(&mut self, printer_id: &str, backend: &str, key: String, value: String) {
        let p_key = format!("{}@{}", printer_id, backend);
        self.printer_settings
            .entry(p_key)
            .or_default()
            .insert(key, value);
    }

    /// Gets a specific setting for a printer (printer-specific or global fallback).
    pub fn get_setting_for(&self, printer_id: &str, backend: &str, key: &str) -> Option<&str> {
        let p_key = format!("{}@{}", printer_id, backend);
        if let Some(val) = self.printer_settings.get(&p_key).and_then(|m| m.get(key)) {
            Some(val.as_str())
        } else {
            self.get_global_setting(key)
        }
    }

    /// Removes all printer-specific settings stored for a printer.
    pub fn remove_settings_for(&mut self, printer_id: &str, backend: &str) {
        let key = format!("{}@{}", printer_id, backend);
        self.printer_settings.remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_empty() {
        let config = PrinterConfig::default();
        assert!(config.last_printer_id.is_none());
        assert!(config.last_backend.is_none());
        assert!(config.global_settings.is_empty());
        assert!(config.printer_settings.is_empty());
        assert!(!config.has_printer());
    }

    #[test]
    fn set_last_printer_works() {
        let mut config = PrinterConfig::new();
        config.set_last_printer("HP-123", "CUPS");
        assert_eq!(config.last_printer_id.as_deref(), Some("HP-123"));
        assert_eq!(config.last_backend.as_deref(), Some("CUPS"));
        assert!(config.has_printer());
    }

    #[test]
    fn global_settings_store_and_fallback() {
        let mut config = PrinterConfig::new();
        config.set_global_setting("copies".to_string(), "2".to_string());
        assert_eq!(config.get_global_setting("copies"), Some("2"));
        assert_eq!(config.get_global_setting("nonexistent"), None);

        // check fallback via get_setting_for
        assert_eq!(
            config.get_setting_for("HP-123", "CUPS", "copies"),
            Some("2")
        );
    }

    #[test]
    fn printer_specific_overrides() {
        let mut config = PrinterConfig::new();
        config.set_global_setting("copies".to_string(), "1".to_string());
        config.set_global_setting("media".to_string(), "Letter".to_string());

        let mut hp_settings = HashMap::new();
        hp_settings.insert("media".to_string(), "A4".to_string());
        config.set_settings_for("HP-123", "CUPS", hp_settings);

        // HP-123 should have media=A4 (overridden) and copies=1 (fallback)
        assert_eq!(
            config.get_setting_for("HP-123", "CUPS", "media"),
            Some("A4")
        );
        assert_eq!(
            config.get_setting_for("HP-123", "CUPS", "copies"),
            Some("1")
        );

        // Update a single setting for HP-123 via set_setting_for
        config.set_setting_for(
            "HP-123",
            "CUPS",
            "sides".to_string(),
            "two-sided-long-edge".to_string(),
        );
        assert_eq!(
            config.get_setting_for("HP-123", "CUPS", "sides"),
            Some("two-sided-long-edge")
        );

        // General settings for HP-123 should be merged
        let merged = config.get_settings_for("HP-123", "CUPS");
        assert_eq!(merged.get("media").map(|s| s.as_str()), Some("A4"));
        assert_eq!(merged.get("copies").map(|s| s.as_str()), Some("1"));
        assert_eq!(
            merged.get("sides").map(|s| s.as_str()),
            Some("two-sided-long-edge")
        );

        // Remove HP-123 settings and check it falls back to Letter
        config.remove_settings_for("HP-123", "CUPS");
        assert_eq!(
            config.get_setting_for("HP-123", "CUPS", "media"),
            Some("Letter")
        );
        assert_eq!(config.get_setting_for("HP-123", "CUPS", "sides"), None);
    }

    #[test]
    fn serde_roundtrip_json() {
        let mut config = PrinterConfig::new();
        config.set_last_printer("Epson", "CUPS");
        config.set_global_setting("copies".to_string(), "3".to_string());

        let mut ep_settings = HashMap::new();
        ep_settings.insert("media".to_string(), "A4".to_string());
        config.set_settings_for("Epson", "CUPS", ep_settings);

        let json = serde_json::to_string(&config).unwrap();
        let loaded: PrinterConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, loaded);
    }
}
