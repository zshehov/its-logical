use its_logical::{
    changes::{
        change::{Apply as _, Change},
        deletion::Deletion,
    },
    knowledge::{self, model::fat_term::FatTerm},
};

use super::{NamedTerm, TermHolder, TermsCache, TwoPhaseTerm};

pub(crate) trait Apply {
    fn apply(&mut self, f: impl Fn(&FatTerm) -> FatTerm);
}

// convenience impl so that a TermsCache can be passed to change applications
impl<T, K> knowledge::store::Get for TermsCache<T, K>
where
    T: NamedTerm,
    K: TwoPhaseTerm,
{
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        self.get(term_name).map(|term| match term {
            TermHolder::Normal(t) => t.term(),
            TermHolder::TwoPhase(t) => t.term(),
        })
    }
}

impl<T, K> TermsCache<T, K>
where
    T: NamedTerm + Apply,
    K: TwoPhaseTerm + Apply,
{
    pub(crate) fn apply_automatic_change(&mut self, change: &Change) {
        if let Some(changed) = self.get_mut(&change.original().meta.term.name) {
            let changed_update = |_: &FatTerm| -> FatTerm { change.changed().to_owned() };
            match changed {
                TermHolder::Normal(t) => t.apply(changed_update),
                TermHolder::TwoPhase(t) => t.apply(changed_update),
            }
        }
        let update_fn = |in_term: &FatTerm| -> FatTerm {
            in_term
                .apply(change)
                .get(&in_term.meta.term.name)
                // the change might not affect the in_term so it needs to be returned as is
                .unwrap_or(in_term)
                .to_owned()
        };
        for term in &mut self.terms {
            match term {
                super::TermHolder::Normal(t) => t.apply(update_fn),
                super::TermHolder::TwoPhase(t) => t.apply(update_fn),
            }
        }
    }

    pub(crate) fn apply_automatic_deletion(&mut self, term: &FatTerm) {
        let changed_by_deletion = term.apply_deletion(self);
        let update = |t: &FatTerm| -> FatTerm {
            term.apply_deletion(t)
                .get(&t.meta.term.name)
                .unwrap_or(t)
                .to_owned()
        };
        for term_name in changed_by_deletion.keys() {
            if let Some(cached_term) = self.get_mut(term_name) {
                match cached_term {
                    TermHolder::Normal(s) => s.apply(update),
                    TermHolder::TwoPhase(s) => s.apply(update),
                }
            }
        }
        self.remove(&term.meta.term.name);
    }
}
