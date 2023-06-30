use std::{cell::RefCell, collections::HashMap, rc::Rc};

use egui::Context;
use tracing::debug;

use crate::{
    changes::{self, ArgsChange},
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
                        term_screen::Output::Deleted(_) => {
                            let deleted_term = term_screen.get_pits().original().extract_term();
                            self.handle_term_deletion(&deleted_term);
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
        let mut opened_affected_term_screens =
            self.handle_with_confirmation(original_term, affected);
        Self::push_updated_loaded_terms(
            original_term,
            arg_changes,
            updated_term,
            &mut opened_affected_term_screens,
        );
    }
    fn handle_with_confirmation(
        &mut self,
        original_term: &FatTerm,
        affected: &[String],
    ) -> OpenedTermScreens<'_> {
        debug!("Need confirmation");

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
        Self::adjust_two_phase_commits(&two_phase_commit, &mut opened_affected_term_screens);
        opened_affected_term_screens
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
        let mut all_updates = changes::propagation::apply(
            &original_term,
            &arg_changes,
            &updated_term,
            loaded_term_screens,
        );

        all_updates.insert(
            original_term.meta.term.name.clone(),
            updated_term.to_owned(),
        );
        Self::push_updated_pits(all_updates, &term_name, loaded_term_screens);
    }

    fn push_updated_with_deletion_loaded_terms(
        original_term: &FatTerm,
        loaded_term_screens: &mut OpenedTermScreens<'_>,
    ) {
        let term_name = original_term.meta.term.name.clone();
        let all_updates = changes::propagation::apply_deletion(&original_term, loaded_term_screens);
        Self::push_updated_pits(all_updates, &term_name, loaded_term_screens);
    }

    fn push_updated_pits(
        updates: HashMap<String, FatTerm>,
        update_source: &str,
        loaded_term_screens: &mut OpenedTermScreens<'_>,
    ) {
        for loaded in loaded_term_screens
            .affected
            .iter_mut()
            .chain(std::iter::once(&mut loaded_term_screens.initiator))
        {
            if let Some(updated) = updates.get(&loaded.name()) {
                loaded.get_pits_mut().0.push_pit(&updated, update_source);
                loaded.choose_pit(loaded.get_pits().len() - 1);
            }
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

    fn automatic_update_loaded_affected(
        &mut self,
        affected: &[String],
        update_pit: impl Fn(&mut TermScreenPIT),
    ) {
        for affected_term_name in affected {
            if let Some(loaded_term_screen) = self.term_tabs.get_mut(&affected_term_name) {
                debug!("Updating {}", affected_term_name);
                let (pits, current) = loaded_term_screen.get_pits_mut();
                pits.iter_mut_pits().for_each(&update_pit);
                if let Some(current) = current {
                    update_pit(current);
                    current.start_changes();
                }
            }
        }
    }
    fn automatic_update_persisted_affected(&mut self, updated: HashMap<String, FatTerm>) {
        for (affected_term_name, affected_updated_term) in updated.into_iter() {
            debug!(
                "Updating {} {:?}",
                affected_term_name, affected_updated_term
            );
            self.terms
                .put(&affected_term_name, affected_updated_term)
                .expect("writing to persistence layer should not fail");
        }
    }

    fn handle_automatic_change_propagation(
        &mut self,
        original_term: &FatTerm,
        arg_changes: &[ArgsChange],
        updated_term: &FatTerm,
        affected: &[String],
    ) {
        let term_name = original_term.meta.term.name.clone();
        debug!("Direct change propagation");
        // update the persisted terms
        let affected_terms = {
            let adapter = TermsAdapter::new(&self.terms);
            changes::propagation::apply(&original_term, &arg_changes, &updated_term, &adapter)
        };
        // [Thought] Maybe move all these details to changes::propagation as a free-standing func
        // that just accepts an implementor of Terms trait

        self.automatic_update_persisted_affected(affected_terms);
        self.terms
            .put(&term_name, updated_term.to_owned())
            .expect("writing to persistence layer should not fail");

        // update the loaded terms
        let updated_term_tab = self.term_tabs.get_mut(&term_name).unwrap();
        *updated_term_tab = TermScreen::new(&updated_term, false);

        let update_pit = |pit: &mut TermScreenPIT| {
            let with_applied = changes::propagation::apply(
                &original_term,
                &arg_changes,
                &updated_term,
                &SingleTerm {
                    term: pit.extract_term(),
                },
            )
            .get(&pit.name())
            .unwrap()
            .to_owned();

            *pit = TermScreenPIT::new(&with_applied);
        };

        self.automatic_update_loaded_affected(affected, update_pit);
    }

    fn handle_automatic_delete_propagation(
        &mut self,
        original_term: &FatTerm,
        affected: &[String],
    ) {
        let term_name = original_term.meta.term.name.clone();
        debug!("Direct delete propagation");
        // update the persisted terms
        let affected_terms =
            changes::propagation::apply_deletion(&original_term, &TermsAdapter::new(&self.terms));
        self.terms.delete(&term_name);
        self.automatic_update_persisted_affected(affected_terms);

        // update the loaded terms
        self.term_tabs.close(&term_name);
        let update_pit = |pit: &mut TermScreenPIT| {
            *pit = TermScreenPIT::new(
                changes::propagation::apply_deletion(
                    &original_term,
                    &SingleTerm {
                        term: pit.extract_term(),
                    },
                )
                .get(&pit.name())
                .unwrap(),
            );
        };

        self.automatic_update_loaded_affected(affected, update_pit);
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

    fn handle_term_deletion(&mut self, term: &FatTerm) {
        if !term.meta.referred_by.is_empty() {
            let mut opened_affected_term_screens = self.handle_with_confirmation(
                term,
                &changes::propagation::affected_from_deletion(term),
            );

            Self::push_updated_with_deletion_loaded_terms(term, &mut opened_affected_term_screens);
        } else {
            self.handle_automatic_delete_propagation(
                term,
                &changes::propagation::affected_from_deletion(term),
            );
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

struct SingleTerm {
    term: FatTerm,
}

impl changes::propagation::Terms for SingleTerm {
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        if term_name == self.term.meta.term.name {
            Some(self.term.clone())
        } else {
            None
        }
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
