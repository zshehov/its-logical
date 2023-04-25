use std::collections::{HashMap, HashSet};

use egui::Context;

use crate::model::fat_term::FatTerm;

mod widgets;

pub struct App {
    term_tabs: TermTabs,
    current_tab: widgets::tabs::Tab,
    terms: HashMap<String, FatTerm>,
    fact_placeholder_state: Vec<String>,
    rule_placeholder_state: Vec<String>,
}

impl App {
    pub fn new(terms: HashMap<String, FatTerm>) -> Self {
        Self {
            fact_placeholder_state: vec![],
            rule_placeholder_state: vec![],
            term_tabs: TermTabs::default(),
            current_tab: widgets::tabs::ask_tab(),
            terms,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        let mut tab_change_occurred = false;
        egui::SidePanel::left("terms_panel").show(ctx, |ui| {
            ui.heading("Terms");
            ui.separator();

            let term_list_selection = widgets::terms_list::show(ui, self.terms.values());

            if let Some(term_name) = term_list_selection {
                self.term_tabs.add(widgets::tabs::Tab {
                    name: term_name.to_owned(),
                    kind: widgets::tabs::TabKind::Term,
                });
                if self.current_tab.name != term_name {
                    tab_change_occurred = true;
                }
                self.current_tab = widgets::tabs::Tab {
                    name: term_name.to_owned(),
                    kind: widgets::tabs::TabKind::Term,
                }
            }
        });
        egui::TopBottomPanel::top("tabs_panel").show(ctx, |ui| {
            tab_change_occurred |= widgets::tabs::show(
                ui,
                &mut self.current_tab,
                self.term_tabs.tabs_vec.iter().cloned(),
            );
            if tab_change_occurred {
                // reset the placeholder
                let selected_term = self.terms.get(&self.current_tab.name).unwrap();
                self.fact_placeholder_state = vec!["".to_string(); selected_term.meta.args.len()];
                self.rule_placeholder_state = vec!["".to_string(); 3];
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| match self.current_tab.kind {
            widgets::tabs::TabKind::Ask => widgets::ask::show(ui),
            widgets::tabs::TabKind::Term => {
                let change = widgets::term::show(
                    ui,
                    self.terms.get(&self.current_tab.name).unwrap(),
                    &mut self.fact_placeholder_state,
                    &mut self.rule_placeholder_state,
                );
                match change {
                    widgets::term::Change::None => {}
                    widgets::term::Change::NewFact => {
                        self.terms
                            .entry(self.current_tab.name.to_string())
                            .and_modify(|t| {
                                let binding = self
                                    .fact_placeholder_state
                                    .iter()
                                    .map(|a| {
                                        if a == "" {
                                            return None;
                                        }
                                        Some(a.to_string())
                                    })
                                    .collect();
                                t.term
                                    .facts
                                    .push(crate::model::term::args_binding::ArgsBinding { binding })
                            });
                        // TODO: make this work - probably this state needs to be pulled up to the
                        // App itself
                        tab_change_occurred = true;
                    }
                    widgets::term::Change::NewRule => todo!(),
                }
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
