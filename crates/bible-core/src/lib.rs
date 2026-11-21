//! Core, UI-free logic for the Bible app: versification, references,
//! notes and highlights, and the church year.
#![allow(clippy::type_complexity)] // lookup tables keyed by tuples

pub mod audio;
pub mod churchyear;
pub mod crossrefs;
pub mod daily;
pub mod fasting;
pub mod intros;
pub mod notes;
pub mod onthisday;
pub mod places;
pub mod plans;
pub mod refs;
pub mod timeline;
pub mod v11n;

pub use refs::Ref;
