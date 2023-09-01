use std::{cell::RefCell, rc::Rc};

use its_logical::{changes::change::Change, knowledge::model::fat_term::FatTerm};

use self::change_handling::two_phase_commit::TwoPhaseCommit;

pub(crate) mod change_handling;
pub(crate) trait NamedTerm {
    fn new(term: FatTerm) -> Self;
    fn name(&self) -> String;
    fn term(&self) -> FatTerm;
}
pub(crate) trait TwoPhaseTerm: NamedTerm {
    type Creator: NamedTerm;
    fn from(creator: Self::Creator) -> Self;

    fn two_phase_commit(&self) -> &Rc<RefCell<TwoPhaseCommit>>;
    fn current_change(&self) -> Change;
}

pub(crate) enum TermHolder<T: NamedTerm, K: TwoPhaseTerm> {
    Normal(T),
    TwoPhase(K),
}

impl<T: NamedTerm, K: TwoPhaseTerm> TermHolder<T, K> {
    fn name(&self) -> String {
        match self {
            TermHolder::Normal(s) => s.name(),
            TermHolder::TwoPhase(s) => s.name(),
        }
    }
}
pub(crate) struct TermsCache<T: NamedTerm, K: TwoPhaseTerm> {
    terms: Vec<TermHolder<T, K>>,
}

impl<T: NamedTerm, K: TwoPhaseTerm> Default for TermsCache<T, K> {
    fn default() -> Self {
        Self {
            terms: Default::default(),
        }
    }
}

// prematurely-optimised aspect of the TermsCache, for when users remember the idx and query directly
// by index to avoid iteration
impl<T: NamedTerm, K: TwoPhaseTerm> TermsCache<T, K> {
    pub(crate) fn find(&self, term_name: &str) -> Option<usize> {
        self.terms.iter().position(|x| x.name() == term_name)
    }

    pub(crate) fn get_by_idx(&self, idx: usize) -> Option<&TermHolder<T, K>> {
        self.terms.get(idx)
    }
    pub(crate) fn get_by_idx_mut(&mut self, idx: usize) -> Option<&mut TermHolder<T, K>> {
        self.terms.get_mut(idx)
    }
}

impl<T, K> TermsCache<T, K>
where
    T: NamedTerm,
    K: TwoPhaseTerm + TwoPhaseTerm<Creator = T>,
{
    pub(crate) fn push(&mut self, term: &FatTerm) {
        self.terms
            .push(TermHolder::Normal(NamedTerm::new(term.clone())));
    }

    pub(crate) fn get(&self, term_name: &str) -> Option<&TermHolder<T, K>> {
        if let Some(term_idx) = self.terms.iter().position(|x| x.name() == term_name) {
            return Some(&self.terms[term_idx]);
        }
        None
    }

    pub(crate) fn get_mut(&mut self, term_name: &str) -> Option<&mut TermHolder<T, K>> {
        if let Some(term_idx) = self.terms.iter().position(|x| x.name() == term_name) {
            return Some(&mut self.terms[term_idx]);
        }
        None
    }

    pub(crate) fn promote(&mut self, term_name: &str) -> Option<&mut K> {
        if let Some(term_idx) = self.terms.iter().position(|x| x.name() == term_name) {
            let screens_len = self.terms.len();
            let to_be_promoted = self.terms.swap_remove(term_idx);
            match to_be_promoted {
                TermHolder::Normal(s) => {
                    self.terms.push(TermHolder::TwoPhase(TwoPhaseTerm::from(s)))
                }
                already_promoted @ TermHolder::TwoPhase(_) => self.terms.push(already_promoted),
            }
            self.terms.swap(term_idx, screens_len - 1);
            return self.terms.get_mut(term_idx).and_then(|t| match t {
                TermHolder::Normal(_) => None,
                TermHolder::TwoPhase(t) => Some(t),
            });
        }
        None
    }

    pub(crate) fn remove(&mut self, term_name: &str) -> Option<TermHolder<T, K>> {
        if let Some(term_idx) = self.terms.iter().position(|x| x.name() == term_name) {
            return Some(self.terms.remove(term_idx));
        }
        None
    }

    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = &TermHolder<T, K>> {
        self.terms.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> impl ExactSizeIterator<Item = &mut TermHolder<T, K>> {
        self.terms.iter_mut()
    }
}
