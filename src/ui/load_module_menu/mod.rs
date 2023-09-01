use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};

use egui::{RichText, Ui};
use git2::build::CheckoutBuilder;
use tracing::debug;

pub(crate) struct LoadModuleMenu {
    to_load_url: String,
    cached_module_names: Vec<String>,
    base_local_dir: PathBuf,
    loading: Option<LoadingProgress>,
}

impl LoadModuleMenu {
    pub(crate) fn new(base_dir: PathBuf) -> Self {
        if !base_dir.is_dir() {
            panic!(
                "provided base dir path is not a directory: {}",
                base_dir.display()
            );
        }

        Self {
            to_load_url: String::new(),
            cached_module_names: Vec::new(),
            base_local_dir: base_dir,
            loading: None,
        }
    }

    fn refresh_module_names(&mut self) {
        self.cached_module_names.clear();
        for base_dir_entry in std::fs::read_dir(&self.base_local_dir)
            .expect("the provided base directory must be traversable")
        {
            let base_dir_entry = base_dir_entry.unwrap();
            if base_dir_entry.file_type().unwrap().is_dir() {
                self.cached_module_names
                    .push(base_dir_entry.file_name().to_str().unwrap().to_string());
            }
        }
    }
}

impl LoadModuleMenu {
    pub(crate) fn show(&mut self, ui: &mut Ui) -> Option<PathBuf> {
        let mut output = None;
        if ui
            .menu_button(RichText::new("load module").italics(), |ui| {
                for module_name in &self.cached_module_names {
                    if ui.small_button(module_name).clicked() {
                        output = Some(self.base_local_dir.join(module_name));
                        ui.close_menu();
                    }
                }

                ui.separator();
                ui.label(RichText::new("Load from:").italics());

                ui.horizontal(|ui| {
                    egui::TextEdit::singleline(&mut self.to_load_url)
                        .frame(true)
                        .clip_text(false)
                        .hint_text("git@github.com:knowledge/yields.power")
                        .show(ui);

                    if !self.to_load_url.is_empty() && ui.button(RichText::new("⏵")).clicked() {
                        let (tx, rx) = mpsc::channel();

                        let repo_name = self
                            .to_load_url
                            .rsplit_once('/')
                            .expect("TODO: no verification yet")
                            .1
                            .trim_end_matches(".git");
                        let local_repo = self.base_local_dir.join(repo_name);
                        let load_url = self.to_load_url.clone();

                        let handle = std::thread::spawn(move || {
                            let mut callbacks = git2::RemoteCallbacks::new();
                            callbacks.credentials(|_url, username_from_url, _allowed_types| {
                                debug!("{}", username_from_url.unwrap());
                                git2::Cred::ssh_key_from_agent("git")
                            });

                            let mut fo = git2::FetchOptions::new();
                            fo.remote_callbacks(callbacks);

                            let mut co = CheckoutBuilder::new();
                            co.progress(move |_, curr, total| {
                                tx.send((curr, total)).unwrap();
                            });

                            let mut builder = git2::build::RepoBuilder::new();
                            builder.fetch_options(fo);
                            builder.with_checkout(co);
                            builder.clone(&load_url, &local_repo);
                        });

                        self.loading = Some(LoadingProgress {
                            rx,
                            handle,
                            current_progress: (0, 0),
                        });
                    }
                });
                let mut loading_finished = false;
                if let Some(progress) = &mut self.loading {
                    ui.ctx().request_repaint();
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Progress);
                    ui.label(format!(
                        "{} / {}",
                        progress.current_progress.0, progress.current_progress.1
                    ));
                    if progress.handle.is_finished() {
                        loading_finished = true;
                    }
                    if let Ok(latest) = progress.rx.try_recv() {
                        progress.current_progress = latest;
                    }
                }

                if loading_finished {
                    let progress = self
                        .loading
                        .take()
                        .expect("there must be a 'loading' if it is finished");
                    progress
                        .handle
                        .join()
                        .expect("TODO: handle errors during downloading");
                    self.refresh_module_names();
                    self.to_load_url.clear();
                }
            })
            .response
            .clicked()
        {
            self.refresh_module_names();
        };
        output
    }
}

struct LoadingProgress {
    rx: Receiver<(usize, usize)>,
    handle: std::thread::JoinHandle<()>,
    current_progress: (usize, usize),
}
