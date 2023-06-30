use std::{cell::RefCell, rc::Rc};

use egui::Context;
use tracing::debug;

use crate::{
    changes, model::fat_term::FatTerm, term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::term_screen::TermScreen,
};

use self::widgets::{
    tabs::{ChosenTab, Tabs},
    term_screen::{self, two_phase_commit::TwoPhaseCommit},
};

mod changes_handling;
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
                let screen_output = egui::CentralPanel::default()
                    .show(ctx, |ui| term_screen.show(ui, &mut self.terms))
                    .inner;

                if let Some(screen_output) = screen_output {
                    match screen_output {
                        term_screen::Output::Changes(changes, updated_term) => {
                            let original_term = term_screen.get_pits().original().extract_term();
                            changes_handling::handle_term_screen_changes(
                                &mut self.term_tabs,
                                &mut self.terms,
                                &original_term,
                                &changes,
                                updated_term,
                            );
                        }
                        term_screen::Output::Deleted(_) => {
                            let deleted_term = term_screen.get_pits().original().extract_term();
                            changes_handling::handle_deletion(
                                &mut self.term_tabs,
                                &mut self.terms,
                                &deleted_term,
                            );
                        }
                        term_screen::Output::FinishTwoPhaseCommit => {
                            let two_phase_commit =
                                Rc::clone(term_screen.two_phase_commit.as_mut().unwrap());

                            let is_deleted = term_screen.in_deletion();
                            self.handle_finish_commit(is_deleted, two_phase_commit);
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

    fn handle_finish_commit(
        &mut self,
        is_delete: bool,
        two_phase_commit: Rc<RefCell<TwoPhaseCommit>>,
    ) {
        debug!("finished commit");

        if two_phase_commit.borrow().waiting_for().len() > 0 {
            debug!("NOT ALL ARE CONFIRMED YET");
        } else {
            debug!("ALL ARE CONFIRMED");
            // TODO: this should be done recursively
            let mut relevant: Vec<String> = two_phase_commit.borrow().iter_approved().collect();
            let origin = two_phase_commit.borrow().origin();

            if !is_delete {
                relevant.push(origin);
            } else {
                self.terms.delete(&origin);
                self.term_tabs.close(&origin);
            }

            for relevant_term_screen in self.term_tabs.borrow_mut(&relevant) {
                let latest_term = relevant_term_screen.extract_term();
                *relevant_term_screen = TermScreen::new(&latest_term, false);
                self.terms
                    .put(&latest_term.meta.term.name.clone(), latest_term);
            }
        }
    }
}

// TODO: this does not need to be public and shouldn't be here
impl changes::propagation::Terms for Tabs {
    fn get(&self, term_name: &str) -> Option<crate::model::fat_term::FatTerm> {
        self.get(term_name).and_then(|t| Some(t.extract_term()))
    }
}

