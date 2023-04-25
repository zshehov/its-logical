#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub mod model;
pub(crate) mod ui;
pub use app::ItsLogicalApp;
