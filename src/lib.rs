#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

pub mod config;
pub mod error;
pub mod events;
pub mod media;
pub mod options;

#[cfg(feature = "zbus-backend")]
pub mod client;
#[cfg(feature = "zbus-backend")]
pub mod proxy;

#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(feature = "ffi")]
pub use ffi::{
    backend::Backend,
    common::{init, version},
    frontend::Frontend,
    job::PrintJob,
    printer::Printer,
    settings::{Media, Options, Settings},
};
