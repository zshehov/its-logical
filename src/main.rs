#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use tracing::Level;

use its_logical::knowledge::store::PersistentTermsWithEngine;

mod app;
mod change_propagation;
mod suggestions;
mod terms_cache;
mod ui;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use std::{env, path::PathBuf};

    use app::ItsLogicalApp;

    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    // what if we wnt to
    let native_options = eframe::NativeOptions::default();
    let knowledge_path = env::var("KNOWLEDGE_PATH").unwrap_or("~/knowledge".to_string());
    let knowledge_path = PathBuf::from(knowledge_path);
    eframe::run_native(
        "It's Logical",
        native_options,
        Box::new(|cc| {
            Box::new(ItsLogicalApp::<PersistentTermsWithEngine>::new(
                cc,
                knowledge_path,
            ))
        }),
    )
}

// when compiling to web using trunk.
#[cfg(target_arch = "wasm32")]
fn main() {
    use its_logical::ItsLogicalApp;
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "the_canvas_id", // hardcode it
            web_options,
            Box::new(|_| Box::new(ItsLogicalApp::new())),
        )
            .await
            .expect("failed to start eframe");
    });
}
