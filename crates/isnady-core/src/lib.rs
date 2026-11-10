//! isnady's core in Rust.
//!
//! The Rust isnady and the Python isnady (github.com/bayramkotan/isnady) are one program written twice: the same data
//! folder, the same database (schema below), the same settings, the same rules — and the same version number. What
//! is ported is listed in PARITY.md; what is not yet ported is done by the Python `iy` on the same data.

pub mod bookmarks;
pub mod books;
pub mod cache;
pub mod config;
pub mod db;
pub mod difflib;
pub mod genealogy;
pub mod grades;
pub mod huffaz;
pub mod i18n;
pub mod isnad;
pub mod learn;
pub mod lives;
pub mod log;
pub mod name_parts;
pub mod names;
pub mod narrators;
pub mod normalize;
pub mod paths;
pub mod people;
pub mod py;
pub mod quran;
// the chain reader is the Python isnady's core/isnad.py and core/narrators.py line for line: their loops over
// indexes, their tuples and their one-off patterns are kept as they are written there
#[allow(
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::regex_creation_in_loops,
    clippy::collapsible_str_replace,
    clippy::wrong_self_convention
)]
pub mod reader;
pub mod readmarks;
pub mod rijal;
pub mod sahaba;
pub mod sahifa;
pub mod scholars;
pub mod search;
pub mod shortcut;
pub mod sira;
pub mod sources;
// the statistics are written step for step as the Python isnady computes them (indexes, the Lanczos constants as
// CPython has them): the same order of operations gives the same figures
#[allow(
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::excessive_precision
)]
pub mod stats;
pub mod tables;
pub mod takhrij;
pub mod works;

/// The version, the same number as the Python isnady's.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The database schema this core reads (the Python isnady's `isnady/data/db.py` SCHEMA_VERSION).
pub const SCHEMA_VERSION: i64 = 8;
