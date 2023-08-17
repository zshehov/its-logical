#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub mod changes;
pub(crate) mod suggestions;
pub(crate) mod ui;
pub use app::ItsLogicalApp;
pub mod knowledge;
