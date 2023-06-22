use std::{cell::RefCell, rc::Rc};

use egui::Context;
use tracing::debug;

use crate::{term_knowledge_base::TermsKnowledgeBase, ui::widgets::term_screen::two_phase_commit};

use self::widgets::{
    tabs::{ChosenTab, Tabs},
    term_screen::{self, two_phase_commit::TwoPhaseCommit},
};

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

                match changes {
                    term_screen::Output::Changed(changes) => {
                        let affected = change_propagator::get_relevant(
                            &term_screen.get_pits().original().extract_term(),
                            &changes,
                        );

                        debug!(
                            "Changes made for {}. Propagating to: {:?}",
                            term_name, affected
                        );
                        if change_propagator::need_confirmation(&changes) {
                            debug!("Changes need confirmation");
                            // TODO: no need for approval from self
                            let two_phase_commit = Rc::clone(
                                term_screen
                                    .two_phase_commit
                                    .get_or_insert(Rc::new(RefCell::new(TwoPhaseCommit::new()))),
                            );

                            two_phase_commit
                                .borrow_mut()
                                .append_approval_from(&affected);

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
                                    let affected_term =
                                        self.term_tabs.get_mut(&affected_term_name).unwrap();

                                    affected_term
                                        .get_pits_mut()
                                        .unwrap()
                                        .push_pit(&affected_updated_term, &term_name);

                                    if affected_term_name != term_name {
                                        affected_term
                                            .two_phase_commit
                                            .get_or_insert(Rc::new(RefCell::new(
                                                TwoPhaseCommit::new(),
                                            )))
                                            .borrow_mut()
                                            .add_waiter_for_approval(Rc::clone(&two_phase_commit));
                                    }
                                }
                            }
                        } else {
                            if let Some(two_phase_commit) = &mut term_screen.two_phase_commit {
                                if two_phase_commit.borrow().waits_for_approval() {
                                    debug!("NOT ALL ARE CONFIRMED YET");
                                } else {
                                    debug!("ALL ARE CONFIRMED");
                                    // TODO: this should be done recursively
                                    let approved: Vec<String> =
                                        two_phase_commit.borrow().iter_approved().collect();

                                    for approved_name in approved {
                                        self.terms.put(&approved_name,
                                                       self.term_tabs.get(&approved_name)
                                                       .expect("all terms part of the 2-phase-commit can't be closed")
                                                       .extract_term());
                                    }
                                }
                            } else {
                                debug!("NOT EVEN A COMMIT AY");
                                let all_changes = change_propagator::apply_changes(
                                    &changes,
                                    &TermsAdapter::new(&self.terms),
                                );
                                for (affected_term_name, affected_updated_term) in all_changes {
                                    self.terms.put(&affected_term_name, affected_updated_term);
                                }
                            }
                        }
                    }
                    term_screen::Output::FinishTwoPhaseCommit => {
                        debug!("finished commit");
                        let two_phase_commit = Rc::clone(
                            term_screen
                                .two_phase_commit
                                .as_ref()
                                .expect("finishing a commit means that there is a commit"),
                        );

                        if two_phase_commit.borrow().waits_for_approval() {
                            debug!("NOT ALL ARE CONFIRMED YET");
                        } else {
                            debug!("ALL ARE CONFIRMED");
                            // TODO: this should be done recursively
                            let approved: Vec<String> =
                                two_phase_commit.borrow().iter_approved().collect();

                            self.terms.put(&term_name, term_screen.extract_term());

                            for approved_name in approved {
                                self.terms.put(
                                    &approved_name,
                                    self.term_tabs
                                        .get(&approved_name)
                                        .expect(
                                            "all terms part of the 2-phase-commit can't be closed",
                                        )
                                        .extract_term(),
                                );
                            }
                        }
                    }
                    term_screen::Output::None => {}
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

struct TermsAdapter<'a, T: TermsKnowledgeBase> {
    incoming: &'a T,
}

impl<'a, T: TermsKnowledgeBase> TermsAdapter<'a, T> {
    fn new(incoming: &'a T) -> Self {
        Self { incoming }
    }
}

impl<'a, T: TermsKnowledgeBase> change_propagator::Terms for TermsAdapter<'a, T> {
    fn get(&self, term_name: &str) -> Option<crate::model::fat_term::FatTerm> {
        self.incoming.get(term_name)
    }
}
