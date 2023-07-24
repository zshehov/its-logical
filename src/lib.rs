#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub mod model;
pub(crate) mod ui;
pub(crate) mod suggestions;
pub mod changes;
pub use app::ItsLogicalApp;
pub mod term_knowledge_base;
