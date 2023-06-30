use std::{cell::RefCell, rc::Rc};

use egui::Context;
use tracing::debug;

use crate::{
    changes, term_knowledge_base::TermsKnowledgeBase, ui::widgets::term_screen::TermScreen,
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
                            changes_handling::finish_commit(
                                &mut self.term_tabs,
                                &mut self.terms,
                                is_deleted,
                                two_phase_commit,
                            );
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
