use std::cell::RefCell;

use egui::Context;

use crate::term_knowledge_base::TermsKnowledgeBase;

use self::widgets::tabs::{ChosenTab, Tabs};

mod change_propagator;
mod widgets;

pub struct App<T: TermsKnowledgeBase> {
    term_tabs: Tabs,
    terms: T,
}

impl<T> App<T>
where
    T: TermsKnowledgeBase,
{
    pub fn new(terms: T) -> Self {
        Self {
            term_tabs: Tabs::default(),
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
                self.term_tabs.add_new_term();
            };
            let term_list_selection = widgets::terms_list::show(ui, self.terms.keys().iter());

            if let Some(term_name) = term_list_selection {
                self.term_tabs.select_with_push(&term_name, &self.terms);
            }
        });

        let chosen_tab = egui::TopBottomPanel::top("tabs_panel")
            .show(ctx, |ui| {
                return self.term_tabs.show(ui);
            })
            .inner;

        match chosen_tab {
            ChosenTab::Term(term_screen) => {
                let term_name = term_screen.name();
                let change_origin = term_screen
                    .get_pits()
                    .change_origin()
                    .unwrap_or(term_name.clone());

                let changes = egui::CentralPanel::default()
                    .show(ctx, |ui| term_screen.show(ui, &mut self.terms))
                    .inner;

                if let Some(changes) = changes {
                    let affected = change_propagator::get_relevant(
                        &term_screen.get_pits().original().extract_term(),
                        &changes,
                    );
                    if change_propagator::need_confirmation(&changes) {
                        if affected.iter().any(|affected_term_name| -> bool {
                            match self.term_tabs.get(&affected_term_name) {
                                Some(affected_opened_term) => {
                                    !affected_opened_term.is_ready_for_change(&change_origin)
                                }
                                None => false,
                            }
                        }) {
                            // TODO: handle change chain disconnection
                        } else {
                            for affected_term_name in affected {
                                if self.term_tabs.get(&affected_term_name).is_none() {
                                    self.term_tabs
                                        .push(&self.terms.get(&affected_term_name).unwrap());
                                }
                            }
                            // now all the affected terms are opened in their respective
                            // screens
                            let all_changes =
                                change_propagator::apply_changes(&changes, &self.term_tabs);
                            for (affected_term_name, affected_updated_term) in all_changes {
                                // TODO: change the name of this method
                                self.term_tabs
                                    .get_mut(&affected_term_name)
                                    .unwrap()
                                    .get_pits_mut()
                                    .unwrap()
                                    .push_pit(&affected_updated_term, &change_origin, &term_name);
                            }
                        }
                    }
                }
            }
            ChosenTab::Ask(_) => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    widgets::ask::show(ui);
                });
            }
        };
    }
}

impl change_propagator::Terms for Tabs {
    fn get(&self, term_name: &str) -> Option<crate::model::fat_term::FatTerm> {
        self.get(term_name).and_then(|t| Some(t.extract_term()))
    }
}
