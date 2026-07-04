//! Raw FFI bindings for the [cpdb-libs](https://github.com/OpenPrinting/cpdb-libs)
//! C library (Common Print Dialog Backends).
//!
//! # Overview
//!
//! `cpdb-sys` is the low-level foundation of the `cpdb-rs` workspace. It
//! uses [`bindgen`](https://crates.io/crates/bindgen) to generate Rust
//! declarations for every C function, type, and constant exposed by
//! `libcpdb` and `libcpdb-frontend`, and wraps them in thin, safe-ish
//! Rust modules.
//!
//! **Most Rust users should depend on [`cpdb-rs`](https://crates.io/crates/cpdb-rs)
//! instead**, either with the `zbus-backend` feature (async, pure-Rust,
//! zero C dependencies) or the `ffi` feature (which re-exports this crate).
//! Reach for `cpdb-sys` directly only when you need access to a C symbol
//! that the higher-level crate hasn't wrapped yet.
//!
//! # Module structure
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`bindings`] | Raw auto-generated `bindgen` output - all `unsafe` |
//! | [`callbacks`] | Safe closure trampolines for `cpdb_printer_callback` and `cpdb_async_callback` |
//! | [`common`] | Helpers for paths, config dirs, and library init (`cpdbInit`) |
//! | [`error`] | [`error::CpdbError`] enum and `Result` alias used by all modules |
//! | [`frontend`] | Safe wrapper around `cpdb_frontend_obj_t` |
//! | [`options`] | [`options::OptionsCollection`] - owned snapshot of a printer's capabilities |
//! | [`printer`] | Safe wrapper around `cpdb_printer_obj_t` |
//! | [`settings`] | Safe wrapper around `cpdb_settings_t` |
//! | [`util`] | Internal C-string helpers and `COptions` array builder |
//!
//! All raw C symbols (functions, types, constants) are additionally
//! re-exported at the crate root via `pub use bindings::*`, so you can
//! write `cpdb_sys::cpdbInit()` instead of `cpdb_sys::bindings::cpdbInit()`.
//!
//! # Safety
//!
//! Everything in [`bindings`] is `unsafe`. The higher-level modules
//! (`frontend`, `printer`, `settings`, ...) encapsulate the most common
//! usage patterns behind safe APIs, but they cannot prevent all misuse -
//! read each module's documentation carefully before calling into the C
//! library directly.
//!
//! # Build requirements
//!
//! `cpdb-sys` links against `libcpdb` and `libcpdb-frontend`. Install
//! the development headers before building:
//!
//! ```text
//! # Debian / Ubuntu
//! sudo apt install libcpdb-dev
//!
//! # Fedora / RHEL
//! sudo dnf install cpdb-libs-devel
//! ```
//!
//! Set `CPDB_LIBS_PATH=<prefix>` if the library is installed to a
//! non-standard location where `pkg-config` cannot find it.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(missing_docs)]

pub mod bindings;
pub mod callbacks;
pub mod common;
pub mod error;
pub mod frontend;
pub mod options;
pub mod printer;
pub mod settings;
pub mod util;

#[allow(unused_imports)]
pub use bindings::*;
