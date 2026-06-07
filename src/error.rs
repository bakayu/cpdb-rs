//! Crate-wide error type and `Result` alias.

use std::ffi::NulError;
use std::str::Utf8Error;
use thiserror::Error;

/// Errors that originate from the cpdb-rs bindings.
///
/// This type is `#[non_exhaustive]` — match arms must include a wildcard
/// so adding variants in future minor releases is not a breaking change.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum CpdbError {
    /// A C function returned `NULL` where a valid pointer was required.
    #[error("Null pointer encountered")]
    NullPointer,

    /// A printer object pointer is invalid or has been released.
    #[error("Invalid printer object")]
    InvalidPrinter,

    /// A lookup (printer, option, media, translation, ...) returned no result.
    #[error("Not found: {0}")]
    NotFound(String),

    /// A printer-side operation failed (set default, accept jobs, ...).
    #[error("Printer error: {0}")]
    PrinterError(String),

    /// A print job submission failed.
    #[error("Print job failed: {0}")]
    JobFailed(String),

    /// A backend-side operation failed.
    #[error("Backend error: {0}")]
    BackendError(String),

    /// A frontend-side operation failed (D-Bus, lifecycle, ...).
    #[error("Frontend error: {0}")]
    FrontendError(String),

    /// A printer option could not be parsed or applied.
    #[error("Option error: {0}")]
    OptionError(String),

    /// A C string returned by cpdb-libs contained invalid UTF-8.
    #[error("Invalid UTF-8 string: {0}")]
    Utf8Error(#[from] Utf8Error),

    /// A Rust string contained an interior NUL byte.
    #[error("Nul byte in string: {0}")]
    NulError(#[from] NulError),

    /// An I/O error bubbled up from std::io.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// An unexpected status code was returned.
    #[error("Invalid status code: {0}")]
    InvalidStatus(i32),

    /// The requested operation is not supported.
    #[error("Unsupported operation")]
    Unsupported,

    /// A D-Bus protocol error occurred.
    #[cfg(feature = "zbus-backend")]
    #[error("D-Bus error: {0}")]
    DbusError(#[from] zbus::Error),

    /// A D-Bus FDO standard error occurred.
    #[cfg(feature = "zbus-backend")]
    #[error("D-Bus FDO error: {0}")]
    FdoError(#[from] zbus::fdo::Error),
}

/// Shorthand `Result` alias used throughout the crate.
pub type Result<T> = std::result::Result<T, CpdbError>;

/// Convert a `cpdb_sys` FFI error into the crate-wide `CpdbError`.
///
/// This allows `?` to work across the FFI boundary when calling functions
/// from `cpdb_sys` modules (e.g. `util`, `printer`) inside `cpdb_rs` code.
#[cfg(feature = "ffi")]
impl From<cpdb_sys::error::CpdbError> for CpdbError {
    fn from(e: cpdb_sys::error::CpdbError) -> Self {
        match e {
            cpdb_sys::error::CpdbError::NullPointer => Self::NullPointer,
            cpdb_sys::error::CpdbError::InvalidPrinter => Self::InvalidPrinter,
            cpdb_sys::error::CpdbError::NotFound(s) => Self::NotFound(s),
            cpdb_sys::error::CpdbError::PrinterError(s) => Self::PrinterError(s),
            cpdb_sys::error::CpdbError::JobFailed(s) => Self::JobFailed(s),
            cpdb_sys::error::CpdbError::BackendError(s) => Self::BackendError(s),
            cpdb_sys::error::CpdbError::FrontendError(s) => Self::FrontendError(s),
            cpdb_sys::error::CpdbError::OptionError(s) => Self::OptionError(s),
            cpdb_sys::error::CpdbError::Utf8Error(e) => Self::Utf8Error(e),
            cpdb_sys::error::CpdbError::NulError(e) => Self::NulError(e),
            cpdb_sys::error::CpdbError::IoError(e) => Self::IoError(e),
            cpdb_sys::error::CpdbError::InvalidStatus(c) => Self::InvalidStatus(c),
            cpdb_sys::error::CpdbError::Unsupported => Self::Unsupported,
            _ => Self::FrontendError(format!("cpdb-sys error: {e}")),
        }
    }
}
