use std::{cell::RefCell, rc::Rc};

use egui::Context;
use tracing::debug;

use crate::{
    changes::{self, propagation::terms_filter::TermsFilter, ArgsChange},
    model::{comment::name_description::NameDescription, fat_term::FatTerm},
    term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::term_screen::{
        term_screen_pit::{TermChange, TermScreenPIT},
        TermScreen,
    },
};

use self::widgets::{
    drag_and_drop::Change,
    tabs::{ChosenTab, Tabs},
    term_screen::{self, two_phase_commit::TwoPhaseCommit},
};

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
                            self.handle_term_screen_changes(&original_term, &changes, updated_term);
                        }
                        term_screen::Output::Deleted(_) => todo!(),
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

    fn handle_term_screen_changes(
        &mut self,
        original_term: &FatTerm,
        term_changes: &[TermChange],
        mut updated_term: FatTerm,
    ) {
        // only argument changes are tough and need special care
        let arg_changes = term_changes
            .iter()
            .find_map(|change| {
                if let TermChange::ArgChanges(arg_changes) = change {
                    return Some(convert_args_changes(&arg_changes));
                }
                None
            })
            .unwrap_or(vec![]);

        // the user doesn't actually finish the updating due to argument
        // changes, so this is done here
        // TODO: this is not idempotent operation
        changes::propagation::finish_self_term(&mut updated_term, &arg_changes);

        let affected = changes::propagation::affected_from_changes(
            &original_term,
            &updated_term,
            &arg_changes,
        );

        if original_term.meta.term.name == "".to_string() {
            // That's a new term - directly apply
            return self.handle_automatic_change_propagation(
                &original_term,
                &arg_changes,
                &updated_term,
                &affected,
            );
        }

        debug!(
            "Changes made for {}. Propagating to: {:?}",
            original_term.meta.term.name, affected
        );

        if arg_changes.is_empty() || affected.len() == 0 {
            self.handle_automatic_change_propagation(
                original_term,
                &arg_changes,
                &updated_term,
                &affected,
            );
        } else {
            self.handle_change_with_confirmation(
                original_term,
                &arg_changes,
                &updated_term,
                &affected,
            );
        }
    }

    fn handle_change_with_confirmation(
        &mut self,
        original_term: &FatTerm,
        arg_changes: &[ArgsChange],
        updated_term: &FatTerm,
        affected: &[String],
    ) {
        debug!("Changes need confirmation");

        let two_phase_commit = Rc::clone(
            self.term_tabs
                .get_mut(&original_term.meta.term.name)
                .expect("a change is coming from an opened term screen")
                .two_phase_commit
                .get_or_insert(Rc::new(RefCell::new(TwoPhaseCommit::new(
                    &original_term.meta.term.name,
                    true,
                )))),
        );
        two_phase_commit.borrow_mut().append_approval_from(affected);

        // TODO: Match instead of unwrap here
        // On failure - remove the two_phase_commit initiated above
        let mut opened_affected_term_screens = self
            .validate_two_phase(
                &two_phase_commit.borrow(),
                &original_term.meta.term.name,
                affected,
            )
            .unwrap();

        Self::push_updated_loaded_terms(
            original_term,
            arg_changes,
            updated_term,
            &mut opened_affected_term_screens,
        );

        Self::adjust_two_phase_commits(&two_phase_commit, &mut opened_affected_term_screens);
    }

    fn adjust_two_phase_commits(
        two_phase_commit: &Rc<RefCell<TwoPhaseCommit>>,
        loaded_related_terms: &mut OpenedTermScreens<'_>,
    ) {
        let origin_name = two_phase_commit.borrow().origin();

        for opened_affected in &mut loaded_related_terms.affected {
            debug!("Adding affected {}", opened_affected.name());
            opened_affected
                .two_phase_commit
                .get_or_insert(Rc::new(RefCell::new(TwoPhaseCommit::new(
                    &origin_name,
                    false,
                ))))
                .borrow_mut()
                .add_approval_waiter(Rc::clone(&two_phase_commit));
        }
    }

    fn push_updated_loaded_terms(
        original_term: &FatTerm,
        arg_changes: &[ArgsChange],
        updated_term: &FatTerm,
        loaded_term_screens: &mut OpenedTermScreens<'_>,
    ) {
        let term_name = original_term.meta.term.name.clone();
        let mut all_updates = {
            let mut terms_cache =
                changes::propagation::terms_filter::with_terms_cache(loaded_term_screens);

            changes::propagation::apply(
                &original_term,
                &arg_changes,
                &updated_term,
                &mut terms_cache,
            );
            terms_cache.all_terms()
        };
        all_updates.insert(
            original_term.meta.term.name.clone(),
            updated_term.to_owned(),
        );

        for opened_affected in loaded_term_screens
            .affected
            .iter_mut()
            .chain(std::iter::once(&mut loaded_term_screens.initiator))
        {
            let affected_name = opened_affected.name();
            opened_affected
                .get_pits_mut()
                .0
                .push_pit(&all_updates.get(&affected_name).unwrap(), &term_name);
            opened_affected.choose_pit(opened_affected.get_pits().len() - 1);
        }
    }

    fn validate_two_phase<'a>(
        &'a mut self,
        two_phase_commit: &TwoPhaseCommit,
        initiator: &str,
        affected: &[String],
    ) -> Option<OpenedTermScreens<'a>> {
        if affected
            .iter()
            .any(|affected_name| match self.term_tabs.get(affected_name) {
                Some(affected_term_screen) => {
                    !affected_term_screen.is_ready_for_change(&two_phase_commit.origin())
                }
                None => false,
            })
        {
            return None;
        }

        for affected_term_name in affected {
            if self.term_tabs.get(&affected_term_name).is_none() {
                self.term_tabs
                    .push(&self.terms.get(&affected_term_name).unwrap());
            }
        }

        let mut with_initiator: Vec<String> = Vec::with_capacity(affected.len() + 1);
        with_initiator.extend_from_slice(affected);
        with_initiator.push(initiator.to_owned());

        let mut all_term_screens = self.term_tabs.borrow_mut(&with_initiator);
        let initiator = all_term_screens.swap_remove(
            all_term_screens
                .iter()
                .position(|x| x.name() == initiator)
                .unwrap(),
        );

        Some(OpenedTermScreens {
            affected: all_term_screens,
            initiator,
        })
    }

    fn handle_automatic_change_propagation(
        &mut self,
        original_term: &FatTerm,
        arg_changes: &[ArgsChange],
        updated_term: &FatTerm,
        affected: &[String],
    ) {
        let term_name = original_term.meta.term.name.clone();
        /*
        if let Some(two_phase_commit) = &mut original_term_screen.two_phase_commit {
            let all_changes = change_propagator::apply_changes_all(
                &changes,
                &original_term,
                &self.term_tabs,
            );
        } else {
            */
        debug!("Direct change propagation");
        // update the persisted terms
        let all_terms = {
            let adapter = TermsAdapter::new(&self.terms);
            let mut lazy_persisted_terms =
                changes::propagation::terms_filter::with_terms_cache(&adapter);
            changes::propagation::apply(
                &original_term,
                &arg_changes,
                &updated_term,
                &mut lazy_persisted_terms,
            );
            lazy_persisted_terms.all_terms()
        };

        for (affected_term_name, affected_updated_term) in all_terms
            .into_iter()
            .chain(std::iter::once((term_name.clone(), updated_term.clone())))
        {
            debug!(
                "Updating {} {:?}",
                affected_term_name, affected_updated_term
            );
            self.terms
                .put(&affected_term_name, affected_updated_term)
                .expect("writing to persistence layer should not fail");
        }

        // update the loaded terms
        let updated_term_tab = self.term_tabs.get_mut(&term_name).unwrap();
        *updated_term_tab = TermScreen::new(&updated_term, false);

        for affected_term_name in affected {
            if let Some(loaded_term_screen) = self.term_tabs.get_mut(&affected_term_name) {
                debug!("updating {}", affected_term_name);
                let (pits, current) = loaded_term_screen.get_pits_mut();
                for pit in pits.iter_mut_pits() {
                    let mut with_applied =
                        changes::propagation::terms_filter::with_single_term(&pit.extract_term());
                    changes::propagation::apply(
                        &original_term,
                        &arg_changes,
                        &updated_term,
                        &mut with_applied,
                    );
                    let with_applied = with_applied
                        .all_terms()
                        .get(affected_term_name)
                        .unwrap()
                        .to_owned();
                    *pit = TermScreenPIT::new(&with_applied);
                }
                if let Some(current) = current {
                    let mut with_applied = changes::propagation::terms_filter::with_single_term(
                        &current.extract_term(),
                    );
                    changes::propagation::apply(
                        &original_term,
                        &arg_changes,
                        &updated_term,
                        &mut with_applied,
                    );
                    let with_applied = with_applied
                        .all_terms()
                        .get(&current.name())
                        .unwrap()
                        .to_owned();
                    *current = TermScreenPIT::new(&with_applied);
                    current.start_changes();
                }
            }
        }
        //}
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

struct OpenedTermScreens<'a> {
    initiator: &'a mut TermScreen,
    affected: Vec<&'a mut TermScreen>,
}

impl<'a> changes::propagation::Terms for OpenedTermScreens<'a> {
    fn get(&self, term_name: &str) -> Option<crate::model::fat_term::FatTerm> {
        if term_name == self.initiator.name() {
            return Some(self.initiator.extract_term());
        }
        for screen in &self.affected {
            if screen.name() == term_name {
                return Some(screen.extract_term());
            }
        }
        None
    }
}

impl changes::propagation::Terms for Tabs {
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

impl<'a, T: TermsKnowledgeBase> changes::propagation::Terms for TermsAdapter<'a, T> {
    fn get(&self, term_name: &str) -> Option<crate::model::fat_term::FatTerm> {
        self.incoming.get(term_name)
    }
}

fn convert_args_changes(input: &[Change<NameDescription>]) -> Vec<changes::ArgsChange> {
    input
        .iter()
        .map(|change| match change {
            Change::Pushed(arg) => changes::ArgsChange::Pushed(arg.name.clone()),
            Change::Moved(moves) => changes::ArgsChange::Moved(moves.to_owned()),
            Change::Removed(idx, arg) => changes::ArgsChange::Removed(*idx, arg.name.clone()),
        })
        .collect()
}
