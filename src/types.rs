//! Shared utility types for cpdb-rs.

use std::fmt;

/// The operational state of a printer.
///
/// These values map to the IPP `printer-state` attribute, which CPDB
/// backends (e.g. CUPS) pass through as string literals on D-Bus.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrinterState {
    /// The printer is ready and not currently processing a job.
    Idle,
    /// The printer is actively processing a job.
    Processing,
    /// The printer queue has been stopped (due to an error, maintenance, or manual pause).
    Stopped,
    /// A state string the library doesn't recognize. Preserved for
    /// forward-compatibility with future or custom CPDB backends.
    Unknown(String),
}

impl PrinterState {
    /// Returns the string representation of the state.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Idle => "idle",
            Self::Processing => "processing",
            Self::Stopped => "stopped",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for PrinterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<String> for PrinterState {
    fn from(s: String) -> Self {
        match s.as_str() {
            "idle" => Self::Idle,
            "processing" => Self::Processing,
            "stopped" => Self::Stopped,
            _ => Self::Unknown(s),
        }
    }
}

impl From<&str> for PrinterState {
    fn from(s: &str) -> Self {
        match s {
            "idle" => Self::Idle,
            "processing" => Self::Processing,
            "stopped" => Self::Stopped,
            _ => Self::Unknown(s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_printer_state_conversions() {
        assert_eq!(PrinterState::from("idle"), PrinterState::Idle);
        assert_eq!(PrinterState::from("processing"), PrinterState::Processing);
        assert_eq!(PrinterState::from("stopped"), PrinterState::Stopped);
        assert_eq!(
            PrinterState::from("paused"),
            PrinterState::Unknown("paused".to_string())
        );

        assert_eq!(PrinterState::from("idle".to_string()), PrinterState::Idle);
        assert_eq!(
            PrinterState::from("testing".to_string()),
            PrinterState::Unknown("testing".to_string())
        );
    }

    #[test]
    fn test_printer_state_display() {
        assert_eq!(format!("{}", PrinterState::Idle), "idle");
        assert_eq!(format!("{}", PrinterState::Processing), "processing");
        assert_eq!(format!("{}", PrinterState::Stopped), "stopped");
        assert_eq!(
            format!("{}", PrinterState::Unknown("offline".to_string())),
            "offline"
        );
    }

    #[test]
    fn test_printer_state_serde() {
        let state = PrinterState::Idle;
        let serialized = serde_json::to_string(&state).unwrap();
        assert_eq!(serialized, "\"idle\"");
        let deserialized: PrinterState = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, state);

        let custom = PrinterState::Unknown("paused".to_string());
        let serialized_custom = serde_json::to_string(&custom).unwrap();

        assert_eq!(serialized_custom, "{\"unknown\":\"paused\"}");
        let deserialized_custom: PrinterState = serde_json::from_str(&serialized_custom).unwrap();
        assert_eq!(deserialized_custom, custom);
    }
}
