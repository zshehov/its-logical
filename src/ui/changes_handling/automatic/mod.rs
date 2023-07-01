use std::collections::HashMap;

use tracing::debug;

use crate::{
    changes,
    model::fat_term::FatTerm,
    term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::{tabs::Tabs, term_screen::term_screen_pit::TermScreenPIT},
};

pub(crate) mod change;
pub(crate) mod deletion;

/// update_persisted  puts all of the terms in the `updated` HashMap into the provided `terms`
/// persistence layer
fn update_persisted(terms: &mut impl TermsKnowledgeBase, updated: HashMap<String, FatTerm>) {
    for (affected_term_name, affected_updated_term) in updated.into_iter() {
        debug!(
            "Updating {} {:?}",
            affected_term_name, affected_updated_term
        );
        terms
            .put(&affected_term_name, affected_updated_term)
            .expect("writing to persistence layer should not fail");
    }
}

/// update_loaded goes through all the currently opened tabs with terms from the affected slice and
/// applies the update_pit fn on each of their Points in time
fn update_loaded(tabs: &mut Tabs, affected: &[String], update_pit: impl Fn(&mut TermScreenPIT)) {
    for affected_term_name in affected {
        if let Some(loaded_term_screen) = tabs.get_mut(&affected_term_name) {
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

pub(crate) struct SingleTerm {
    term: FatTerm,
}

impl SingleTerm {
    pub(crate) fn new(term: FatTerm) -> Self {
        Self { term }
    }
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
