use std::collections::HashSet;

use egui::Context;

use crate::term_knowledge_base::TermsKnowledgeBase;

use self::widgets::{tabs::Tab, term_screen::TermScreen};

mod widgets;

enum CentralPanelContent {
    None,
    AskScreen,
    TermScreen(widgets::term_screen::TermScreen),
}

pub struct App<T: TermsKnowledgeBase> {
    term_tabs: TermTabs,
    current_tab: widgets::tabs::Tab,
    terms: T,
    central_panel: CentralPanelContent,
}

const NEW_TAB_NAME: &str = "New term";

impl<T> App<T>
where
    T: TermsKnowledgeBase,
{
    pub fn new(terms: T) -> Self {
        Self {
            term_tabs: TermTabs::default(),
            central_panel: CentralPanelContent::None,
            current_tab: widgets::tabs::ask_tab(),
            terms,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        egui::SidePanel::left("terms_panel").show(ctx, |ui| {
            ui.heading("Terms");
            ui.separator();

            if ui
                .button(egui::RichText::new("Add term").underline().strong())
                .clicked()
            {
                self.central_panel = CentralPanelContent::TermScreen(TermScreen::with_new_term());
                let new_term_tab = Tab {
                    name: NEW_TAB_NAME.to_string(),
                    kind: widgets::tabs::TabKind::Term,
                };
                self.current_tab = new_term_tab.clone();
                self.term_tabs.add(new_term_tab);
            };
            let term_list_selection = widgets::terms_list::show(ui, self.terms.keys().iter());

            if let Some(term_name) = term_list_selection {
                self.term_tabs.add(widgets::tabs::Tab {
                    name: term_name.to_owned(),
                    kind: widgets::tabs::TabKind::Term,
                });
                if self.current_tab.name != term_name {
                    // TODO: rework this - the terms list should communicate a change to tabs
                    // widget, then the tab widget handles this out of the box
                    self.central_panel = CentralPanelContent::TermScreen(TermScreen::new(
                        &self.terms.get(&term_name).unwrap().clone(),
                    ))
                }
                self.current_tab = widgets::tabs::Tab {
                    name: term_name.to_owned(),
                    kind: widgets::tabs::TabKind::Term,
                }
            }
        });
        egui::TopBottomPanel::top("tabs_panel").show(ctx, |ui| {
            let tab_change = widgets::tabs::show(
                ui,
                &mut self.current_tab,
                self.term_tabs.tabs_vec.iter().cloned(),
            );
            match tab_change {
                widgets::tabs::TabChange::None => {}
                widgets::tabs::TabChange::ToAskTab => {
                    self.central_panel = CentralPanelContent::AskScreen;
                }
                widgets::tabs::TabChange::ToTermTab => {
                    self.central_panel = CentralPanelContent::TermScreen(TermScreen::new(
                        &self.terms.get(&self.current_tab.name).unwrap().clone(),
                    ))
                }
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| match &mut self.central_panel {
            CentralPanelContent::None => widgets::ask::show(ui),
            CentralPanelContent::AskScreen => widgets::ask::show(ui),
            CentralPanelContent::TermScreen(term_screen) => {
                let change = term_screen.show(ui, &mut self.terms);
                match change {
                    widgets::term_screen::Change::None => {}
                    widgets::term_screen::Change::TermChange(updated_term) => {
                        if self.current_tab.name == NEW_TAB_NAME {
                            self.terms
                                .put(&updated_term.meta.term.name, updated_term.clone());
                        } else {
                            self.terms.edit(&self.current_tab.name, &updated_term);
                        }
                        self.term_tabs
                            .rename(&self.current_tab.name, &updated_term.meta.term.name);
                        self.current_tab.name = updated_term.meta.term.name;
                    }
                    widgets::term_screen::Change::DeletedTerm => {
                        self.term_tabs.remove(&self.current_tab.name);
                        self.current_tab = widgets::tabs::ask_tab();
                        self.central_panel = CentralPanelContent::AskScreen;
                    },
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
    fn rename(&mut self, from: &str, to: &str) {
        if let Some(item) = self.tabs_vec.iter_mut().find(|x| x.name == from) {
            item.name = to.to_owned();
        }
        self.tabs_set.remove(from);
        self.tabs_set.insert(to.to_owned());
    }
    fn remove(&mut self, tab_name: &str) {
        if let Some(item_idx) = self.tabs_vec.iter().position(|x| x.name == tab_name) {
            self.tabs_vec.remove(item_idx);
        }
        self.tabs_set.remove(tab_name);
    }
}
