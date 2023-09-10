use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use its_logical::{
    changes::{
        change::{Apply as _, ArgsChange, Change},
        deletion::Deletion,
    },
    knowledge::{self, model::fat_term::FatTerm},
};

use super::{two_phase_commit::TwoPhaseCommit, NamedTerm, TermsCache, TwoPhaseTerm};

pub(crate) trait Apply {
    fn push_for_confirmation(
        &mut self,
        arg_changes: &[ArgsChange],
        resulting_term: &FatTerm,
        source: &str,
    );
}

impl<T, K> TermsCache<T, K>
where
    T: NamedTerm,
    K: TwoPhaseTerm<Creator = T> + Apply,
{
    // applies a change that should be confirmed to all potentially
    // affected `super::TermHolder::TwoPhase` entries. Meaning that all
    pub(crate) fn apply_for_confirmation_change(
        &mut self,
        // the knowledge::store::Get is needed as the change might affect terms that are not yet cached in
        // the TermsCache, so they would need to be cached during this call
        store: &impl knowledge::store::Get,
        change: &Change,
    ) -> Result<(), &'static str> {
        let (mentioned, referred_by) = change.affects();
        let mut affected_by_change = HashSet::with_capacity(mentioned.len() + referred_by.len());
        affected_by_change.extend(mentioned);
        affected_by_change.extend(referred_by);
        let affected_by_change: Vec<String> = affected_by_change.into_iter().collect();

        if !self.are_ready_for_change(&affected_by_change) {
            return Err("There is a term that is not ready to be included in a 2 phase commit");
        }

        self.push_affected(&affected_by_change, store);

        let all_affected_changed = self.apply(change);

        let original = change.original();

        if self.get(&original.meta.term.name).is_none() {
            self.push(original);
        }

        let change_source_two_phase_commit = {
            let change_source = self
                .promote(&original.meta.term.name)
                .expect("guaranteed to be opened above");
            change_source.push_for_confirmation(
                change.arg_changes(),
                change.changed(),
                &original.meta.term.name,
            );
            change_source.two_phase_commit().to_owned()
        };

        self.push_to_changed(
            &original.meta.term.name,
            &change_source_two_phase_commit,
            store,
            all_affected_changed,
        );
        Ok(())
    }

    pub(crate) fn apply_for_confirmation_delete(
        &mut self,
        deleted_term: &FatTerm,
        store: &impl knowledge::store::Get,
    ) -> Result<(), &'static str> {
        let affected_by_deletion = deleted_term.affects();

        if !self.are_ready_for_change(affected_by_deletion) {
            return Err("There is a term that is not ready to be included in a 2 phase commit");
        }

        self.push_affected(affected_by_deletion, store);

        let changed_by_deletion = deleted_term.apply_deletion(self);
        let deleted_two_phase_commit = self
            .promote(&deleted_term.meta.term.name)
            .expect("it must be opened as it was just deleted")
            .two_phase_commit()
            .to_owned();

        self.push_to_changed(
            &deleted_term.meta.term.name,
            &deleted_two_phase_commit,
            store,
            changed_by_deletion,
        );
        Ok(())
    }

    fn are_ready_for_change(&self, affected: &[String]) -> bool {
        affected
            .iter()
            .all(|affected_name| match self.get(affected_name) {
                // TODO: check if ready for change
                Some(_) => true,
                None => true,
            })
    }

    fn push_affected(&mut self, affected: &[String], store: &impl knowledge::store::Get) {
        for affected_term in affected {
            if self.get(affected_term).is_none() {
                store.get(affected_term).map(|t| {
                    self.push(&t);
                    
                });
            }
        }
    }

    fn push_to_changed(
        &mut self,
        source_name: &str,
        source_two_phase_commit: &Rc<RefCell<TwoPhaseCommit>>,
        store: &impl knowledge::store::Get,
        changed: HashMap<String, FatTerm>,
    ) {
        for (term_name, changed_term) in changed {
            if self.get(&term_name).is_none() {
                self.push(
                    &store
                        .get(&term_name)
                        .expect("this term has come from the knowledge store"),
                );
            }

            let changed_two_phase_commit = self.promote(&term_name).expect("term was just pushed");

            changed_two_phase_commit.push_for_confirmation(&[], &changed_term, source_name);
            super::fix_approvals(
                changed_two_phase_commit.two_phase_commit(),
                source_two_phase_commit,
            )
        }
    }
}
