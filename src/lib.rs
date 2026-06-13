//! Crate-level documentation for `cpdb-rs`.
//!
//! This library provides a native async Rust client for the
//! [Common Print Dialog Backends (CPDB)](https://github.com/OpenPrinting/cpdb-libs)
//! system. It communicates with CPDB backends (e.g. `cpdb-backend-cups`)
//! over D-Bus using [`zbus`], requiring no C dependencies.
//!
//! # Features
//!
//! - `zbus-backend` *(default)* - Native async D-Bus client via [`CpdbClient`].
//!   *(Note: This uses `zbus` with the `tokio` runtime feature exclusively. Other async runtimes like `async-std` are not currently supported by the high-level client).*
//! - `ffi` - Legacy synchronous C FFI bindings via `cpdb-libs`.
//!
//! # Quick start
//!
//! ```no_run
//! # #[cfg(feature = "zbus-backend")]
//! # async fn example() -> cpdb_rs::Result<()> {
//! use cpdb_rs::CpdbClient;
//!
//! let client = CpdbClient::new().await?;
//! let printers = client.get_all_printers().await?;
//!
//! for p in &printers {
//!     println!("{} [{}] - {}", p.name, p.id, p.make_model);
//! }
//!
//! if let Some(p) = printers.first() {
//!     let (options, media) = client
//!         .get_printer_details(&p.id, &p.backend)
//!         .await?;
//!     println!("{} options, {} media sizes", options.len(), media.len());
//! }
//! # Ok(())
//! # }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

pub mod error;
pub mod options;

#[cfg(feature = "zbus-backend")]
pub mod client;
#[cfg(feature = "zbus-backend")]
pub mod config;
#[cfg(feature = "zbus-backend")]
pub mod events;
#[cfg(feature = "zbus-backend")]
pub mod media;
#[cfg(feature = "zbus-backend")]
pub mod proxy;
#[cfg(feature = "zbus-backend")]
pub use client::CpdbClient;
#[cfg(feature = "zbus-backend")]
pub use config::PrinterConfig;
#[cfg(feature = "zbus-backend")]
pub use events::{DiscoveryEvent, PrinterSnapshot};
#[cfg(feature = "zbus-backend")]
pub use media::{MarginInfo, MediaCollection, MediaInfo};

// Re-export core types for convenience.
pub use error::{CpdbError, Result};
pub use options::{OptionInfo, OptionsCollection};

#[cfg(feature = "ffi")]
pub use cpdb_sys as ffi;

#[cfg(feature = "ffi")]
pub use cpdb_sys::callbacks::PrinterUpdate;
#[cfg(feature = "ffi")]
pub use cpdb_sys::{
    common::{
        absolute_path, concat_path, concat_sep, init, option_group, system_config_dir,
        user_config_dir, version,
    },
    frontend::Frontend,
    printer::{
        Margin, Margins, MediaSize, PrintFdHandle, PrintSocketHandle, Printer, TranslationMap,
    },
    settings::{Media, Options, Settings},
};
