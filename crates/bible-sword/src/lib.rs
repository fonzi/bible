//! A GPL-3 Rust implementation of the SWORD module formats used by
//! CrossWire and compatible repositories: reading installed modules and
//! installing new ones from raw HTTPS repositories.
#![allow(clippy::type_complexity)] // lookup tables keyed by tuples

pub mod conf;
pub mod genbook;
pub mod install;
pub mod ld;
pub mod markup;
pub mod module;

pub use install::{RemoteModule, Source};
pub use markup::Span;
pub use module::{Kind, Library, Module};
