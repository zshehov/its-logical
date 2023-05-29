use std::collections::HashSet;

use egui::Context;

use crate::term_knowledge_base::TermsKnowledgeBase;

use self::widgets::{tabs::ChosenTab, term_screen::TermScreen};

mod widgets;

pub struct App<T: TermsKnowledgeBase> {
    term_tabs: TermTabs,
    terms: T,
}

impl<T> App<T>
where
    T: TermsKnowledgeBase,
{
    pub fn new(terms: T) -> Self {
        Self {
            term_tabs: TermTabs::default(),
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
                self.term_tabs.tabs_vec.push(TermScreen::with_new_term());
                self.term_tabs.tabs_vec.select("");
            };
            let term_list_selection = widgets::terms_list::show(ui, self.terms.keys().iter());

            if let Some(term_name) = term_list_selection {
                self.term_tabs.select(&term_name, &self.terms);
            }
        });

        let chosen_tab = egui::TopBottomPanel::top("tabs_panel")
            .show(ctx, |ui| {
                return self.term_tabs.tabs_vec.show(ui);
            })
            .inner;

        match chosen_tab {
            ChosenTab::Term(term_screen) => {
                let change = egui::CentralPanel::default()
                    .show(ctx, |ui| term_screen.show(ui, &mut self.terms))
                    .inner;

                match change {
                    widgets::term_screen::Change::None => {}
                    widgets::term_screen::Change::TermChange {
                        original_name,
                        updated_term,
                    } => {
                        self.term_tabs
                            .rename(&original_name, &updated_term.meta.term.name);
                        self.terms.put(&original_name, updated_term);
                    }
                    widgets::term_screen::Change::DeletedTerm(term_name) => {
                        self.term_tabs.remove(&term_name);
                    }
                };
            }
            ChosenTab::Ask(_) => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    widgets::ask::show(ui);
                });
            }
        };
    }
}

#[derive(Default)]
struct TermTabs {
    tabs_vec: widgets::tabs::TabsState,
    tabs_set: HashSet<String>,
}

impl TermTabs {
    fn select<T: TermsKnowledgeBase>(&mut self, term_name: &str, terms: &T) {
        if self.tabs_set.insert(term_name.to_string()) {
            self.tabs_vec
                .push(TermScreen::new(&terms.get(&term_name).unwrap().clone()));
        }
        self.tabs_vec.select(term_name);
    }

    fn rename(&mut self, from: &str, to: &str) {
        self.tabs_set.remove(from);
        self.tabs_set.insert(to.to_owned());
    }
    fn remove(&mut self, tab_name: &str) {
        self.tabs_vec.remove(tab_name);
        self.tabs_set.remove(tab_name);
    }
}
