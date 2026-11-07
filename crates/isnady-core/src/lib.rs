//! isnady's core in Rust.
//!
//! The Rust isnady and the Python isnady (github.com/bayramkotan/isnady) are one program written twice: the same data
//! folder, the same database (schema below), the same settings, the same rules — and the same version number. What
//! is ported is listed in PARITY.md; what is not yet ported is done by the Python `iy` on the same data.

pub mod bookmarks;
pub mod books;
pub mod config;
pub mod db;
pub mod difflib;
pub mod grades;
pub mod i18n;
pub mod isnad;
pub mod lives;
pub mod name_parts;
pub mod names;
pub mod narrators;
pub mod normalize;
pub mod paths;
pub mod people;
pub mod py;
pub mod quran;
pub mod readmarks;
pub mod scholars;
pub mod search;
pub mod tables;
pub mod takhrij;
pub mod works;

/// The version, the same number as the Python isnady's.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The database schema this core reads (the Python isnady's `isnady/data/db.py` SCHEMA_VERSION).
pub const SCHEMA_VERSION: i64 = 8;
