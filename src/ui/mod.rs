use std::collections::{HashMap, HashSet};

use egui::Context;

use crate::model::term::Term;

mod widgets;

pub struct App {
    term_tabs: TermTabs,
    current_tab: widgets::tabs::Tab,
    terms: HashMap<String, Term>,
}

impl App {
    pub fn new(
        terms: HashMap<String, Term>,
    ) -> Self {
        Self {
            term_tabs: TermTabs::default(),
            current_tab: widgets::tabs::Tab{
                name: "Ask".to_owned(),
                kind: widgets::tabs::TabKind::Ask,
            },
            terms,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        egui::SidePanel::left("terms_panel").show(ctx, |ui| {
            ui.heading("Terms");
            ui.separator();

            let term_list_selection = widgets::terms_list::show(ui, self.terms.values());

            if let Some(term_name) = term_list_selection {
                self.term_tabs.add(widgets::tabs::Tab {
                    name: term_name.to_owned(),
                    kind: widgets::tabs::TabKind::Term,
                });
                self.current_tab = widgets::tabs::Tab {
                    name: term_name.to_owned(),
                    kind: widgets::tabs::TabKind::Term,
                }
            }
        });
        egui::TopBottomPanel::top("tabs_panel").show(ctx, |ui| {
            widgets::tabs::show(
                ui,
                &mut self.current_tab,
                self.term_tabs.tabs_vec.iter().cloned(),
            );
        });
        egui::CentralPanel::default().show(ctx, |ui| match self.current_tab.kind {
            widgets::tabs::TabKind::Ask => widgets::ask::show(ui),
            widgets::tabs::TabKind::Term => {
                widgets::term::show(ui, self.terms.get(&self.current_tab.name).unwrap());
            }
        });
    }
}

#[derive(Default)]
struct TermTabs {
    tabs_vec: Vec<widgets::tabs::Tab>,
    tabs_set: HashSet<String>,
}

impl TermTabs {
    fn add(&mut self, tab: widgets::tabs::Tab) {
        if self.tabs_set.insert(tab.name.to_owned()) {
            self.tabs_vec.push(tab);
        }
    }
}
