use its_logical::{
    changes::{
        change::{Apply as _, ArgsChange, Change},
        deletion::Deletion,
    },
    knowledge::{self, model::fat_term::FatTerm},
};
use tracing::debug;

use super::{NamedTerm, TermsCache, TwoPhaseTerm};

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
        knowledge_store: &impl knowledge::store::Get,
        change: &Change,
    ) -> Result<(), &'static str> {
        let all_affected = knowledge_store.apply(change);
        if all_affected
            .keys()
            .any(|affected_name| match self.get(affected_name) {
                // TODO: check if ready for change
                Some(_) => false,
                None => false,
            })
        {
            return Err("There is a term that is not ready to be included in a 2 phase commit");
        }
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

        for (name, term) in all_affected {
            if self.get(&name).is_none() {
                self.push(&term);
            }
            debug!("fixing {}", name);
            if let Some(two_phase) = self.promote(&name) {
                let term = two_phase.term();

                if let Some(after_change) = term.apply(change).get(&term.meta.term.name) {
                    two_phase.push_for_confirmation(
                        change.arg_changes(),
                        after_change,
                        &original.meta.term.name,
                    );
                    super::fix_approvals(
                        two_phase.two_phase_commit(),
                        &change_source_two_phase_commit,
                    );
                }
            };
        }
        Ok(())
    }

    pub(crate) fn apply_for_confirmation_delete(
        &mut self,
        deleted_term: &FatTerm,
        store: &impl knowledge::store::Get,
    ) {
        let changed_by_deletion = deleted_term.apply_deletion(store);
        let deleted_two_phase_commit = self
            .promote(&deleted_term.meta.term.name)
            .expect("it must be opened as it was just deleted")
            .two_phase_commit()
            .to_owned();

        for (term_name, changed_term) in changed_by_deletion {
            if self.get(&term_name).is_none() {
                self.push(
                    &store
                        .get(&term_name)
                        .expect("this term has come from the knowledge store"),
                );
            }

            let affected_by_deletion_two_phase_commit =
                self.promote(&term_name).expect("term was just pushed");

            affected_by_deletion_two_phase_commit.push_for_confirmation(
                &[],
                &changed_term,
                &deleted_term.meta.term.name,
            );
            super::fix_approvals(
                affected_by_deletion_two_phase_commit.two_phase_commit(),
                &deleted_two_phase_commit,
            )
        }
    }
}
