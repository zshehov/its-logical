use std::{cell::RefCell, rc::Rc};

use egui::Context;
use tracing::debug;

use crate::{
    model::fat_term::FatTerm,
    term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::term_screen::{term_screen_pit::TermScreenPIT, two_phase_commit, TermScreen},
};

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
                        let original_term = term_screen.get_pits().original().extract_term();
                        let affected = change_propagator::get_relevant(&original_term, &changes);

                        debug!(
                            "Changes made for {}. Propagating to: {:?}",
                            term_name, affected
                        );
                        if change_propagator::need_confirmation(&changes) {
                            debug!("Changes need confirmation");
                            let two_phase_commit =
                                Rc::clone(term_screen.two_phase_commit.get_or_insert(Rc::new(
                                    RefCell::new(TwoPhaseCommit::new(true)),
                                )));

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
                                let all_changes = change_propagator::apply_changes(
                                    &changes,
                                    &original_term,
                                    &self.term_tabs,
                                );
                                for (affected_term_name, affected_updated_term) in all_changes {
                                    // TODO: change the name of this method
                                    let affected_term =
                                        self.term_tabs.get_mut(&affected_term_name).unwrap();

                                    affected_term
                                        .get_pits_mut()
                                        .0
                                        .push_pit(&affected_updated_term, &term_name);
                                    affected_term.choose_pit(affected_term.get_pits().len() - 1);

                                    if affected_term_name != term_name {
                                        affected_term
                                            .two_phase_commit
                                            .get_or_insert(Rc::new(RefCell::new(
                                                TwoPhaseCommit::new(false),
                                            )))
                                            .borrow_mut()
                                            .add_approval_waiter(Rc::clone(&two_phase_commit));
                                    }
                                }
                            }
                        } else {
                            if let Some(two_phase_commit) = &mut term_screen.two_phase_commit {
                                if two_phase_commit.borrow().waiting_for().len() > 0 {
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
                                // update the persisted terms
                                let all_changes = change_propagator::apply_changes(
                                    &changes,
                                    &original_term,
                                    &TermsAdapter::new(&self.terms),
                                );
                                for (affected_term_name, affected_updated_term) in all_changes {
                                    self.terms.put(&affected_term_name, affected_updated_term);
                                }

                                // update the loaded terms
                                for affected_term_name in
                                    affected.iter().chain(std::iter::once(&term_name))
                                {
                                    if let Some(loaded_term_screen) =
                                        self.term_tabs.get_mut(&affected_term_name)
                                    {
                                        debug!("updating {}", affected_term_name);
                                        let (pits, current) = loaded_term_screen.get_pits_mut();
                                        for pit in pits.iter_mut_pits() {
                                            let with_applied =
                                                change_propagator::apply_single_changes(
                                                    &changes,
                                                    &original_term,
                                                    &pit.extract_term(),
                                                );
                                            *pit = TermScreenPIT::new(&with_applied, false);
                                        }
                                        if let Some(current) = current {
                                            let with_applied =
                                                change_propagator::apply_single_changes(
                                                    &changes,
                                                    &original_term,
                                                    &current.extract_term(),
                                                );
                                            *current = TermScreenPIT::new(&with_applied, true);
                                        }
                                    }
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

                        if two_phase_commit.borrow().waiting_for().len() > 0 {
                            debug!("NOT ALL ARE CONFIRMED YET");
                        } else {
                            debug!("ALL ARE CONFIRMED");
                            // TODO: this should be done recursively
                            let approved: Vec<String> =
                                two_phase_commit.borrow().iter_approved().collect();

                            let latest_term_version = term_screen.extract_term();
                            *term_screen = TermScreen::new(&latest_term_version, false);
                            self.terms.put(&term_name, latest_term_version);

                            for approved_name in approved {
                                let latest_term_version = self
                                    .term_tabs
                                    .get(&approved_name)
                                    .expect("all terms part of the 2-phase-commit can't be closed")
                                    .extract_term();

                                *self.term_tabs.get_mut(&approved_name).unwrap() =
                                    TermScreen::new(&latest_term_version, false);
                                self.terms.put(&approved_name, latest_term_version);
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
