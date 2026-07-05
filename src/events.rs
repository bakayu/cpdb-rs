//! Discovery events emitted by CPDB backends.
//!
//! These map directly to the D-Bus signals defined in the
//! `org.openprinting.PrintBackend` interface.

use crate::types::PrinterState;

/// An event emitted during printer discovery or state monitoring.
///
/// Marked `#[non_exhaustive]` so a future backend event kind can be
/// added without breaking downstream matches — always include a `_`
/// arm.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiscoveryEvent {
    /// A printer was discovered or re-announced.
    PrinterAdded(PrinterSnapshot),
    /// A printer was removed from the backend.
    PrinterRemoved {
        /// The printer's unique ID.
        id: String,
        /// The backend that reported the removal.
        backend: String,
    },
    /// A printer's state or accepting-jobs status changed.
    PrinterStateChanged {
        /// The printer's unique ID.
        id: String,
        /// The backend that reported the change.
        backend: String,
        /// The new state.
        state: PrinterState,
        /// Whether the printer is accepting new jobs.
        accepting_jobs: bool,
    },
}

/// Snapshot of a printer's identity and status at a point in time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PrinterSnapshot {
    /// The backend-assigned unique printer ID.
    pub id: String,
    /// The human-readable display name.
    pub name: String,
    /// A free-form description of the printer.
    pub info: String,
    /// Physical location string as reported by the backend.
    pub location: String,
    /// Make and model string (e.g. `"HP LaserJet Pro"`).
    pub make_model: String,
    /// Current state (e.g. idle, processing, stopped).
    pub state: PrinterState,
    /// Whether the printer is currently accepting new jobs.
    pub accepting_jobs: bool,
    /// The backend that owns this printer (e.g. `"CUPS"`).
    pub backend: String,
}

impl PrinterSnapshot {
    /// Returns `true` when the printer is idle and accepting jobs.
    pub fn is_ready(&self) -> bool {
        self.state == PrinterState::Idle && self.accepting_jobs
    }
}

impl std::fmt::Display for PrinterSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}] ({})", self.name, self.id, self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot() -> PrinterSnapshot {
        PrinterSnapshot {
            id: "HP-LaserJet-Pro".to_string(),
            name: "HP LaserJet Pro".to_string(),
            info: "Office printer".to_string(),
            location: "Room 42".to_string(),
            make_model: "HP LaserJet Pro MFP".to_string(),
            state: PrinterState::Idle,
            accepting_jobs: true,
            backend: "CUPS".to_string(),
        }
    }

    #[test]
    fn snapshot_is_ready_when_idle_and_accepting() {
        let snap = sample_snapshot();
        assert!(snap.is_ready());
    }

    #[test]
    fn snapshot_not_ready_when_busy() {
        let mut snap = sample_snapshot();
        snap.state = PrinterState::Processing;
        assert!(!snap.is_ready());
    }

    #[test]
    fn snapshot_not_ready_when_not_accepting() {
        let mut snap = sample_snapshot();
        snap.accepting_jobs = false;
        assert!(!snap.is_ready());
    }

    #[test]
    fn snapshot_clones_correctly() {
        let snap = sample_snapshot();
        let clone = snap.clone();
        assert_eq!(snap.id, clone.id);
        assert_eq!(snap.backend, clone.backend);
    }

    #[test]
    fn discovery_event_printer_added() {
        let event = DiscoveryEvent::PrinterAdded(sample_snapshot());
        assert!(matches!(event, DiscoveryEvent::PrinterAdded(_)));
    }

    #[test]
    fn discovery_event_printer_removed() {
        let event = DiscoveryEvent::PrinterRemoved {
            id: "HP-123".to_string(),
            backend: "CUPS".to_string(),
        };
        match &event {
            DiscoveryEvent::PrinterRemoved { id, backend } => {
                assert_eq!(id, "HP-123");
                assert_eq!(backend, "CUPS");
            }
            _ => panic!("Expected PrinterRemoved"),
        }
    }

    #[test]
    fn discovery_event_state_changed() {
        let event = DiscoveryEvent::PrinterStateChanged {
            id: "HP-123".to_string(),
            backend: "CUPS".to_string(),
            state: PrinterState::Processing,
            accepting_jobs: true,
        };
        match &event {
            DiscoveryEvent::PrinterStateChanged {
                state,
                accepting_jobs,
                ..
            } => {
                assert_eq!(state, &PrinterState::Processing);
                assert!(*accepting_jobs);
            }
            _ => panic!("Expected PrinterStateChanged"),
        }
    }

    #[test]
    fn events_are_clone() {
        let event = DiscoveryEvent::PrinterAdded(sample_snapshot());
        let clone = event.clone();
        assert!(matches!(clone, DiscoveryEvent::PrinterAdded(_)));
    }

    #[test]
    fn snapshot_display() {
        let snap = sample_snapshot();
        assert_eq!(
            format!("{}", snap),
            "HP LaserJet Pro [HP-LaserJet-Pro] (idle)"
        );
    }

    #[test]
    fn snapshot_equality_and_hashing() {
        use std::collections::HashSet;
        let snap1 = sample_snapshot();
        let snap2 = sample_snapshot();
        assert_eq!(snap1, snap2);

        let mut set = HashSet::new();
        set.insert(snap1.clone());
        assert!(set.contains(&snap2));
    }
}
