use tracing::debug;

use crate::{
    changes::{self, ArgsChange},
    model::fat_term::FatTerm,
    term_knowledge_base::{GetKnowledgeBase, PutKnowledgeBase},
};

pub(crate) trait Loaded {
    fn update_with(&mut self, term_name: &str, updator: impl Fn(&FatTerm) -> FatTerm);
}

mod loaded_impl;

pub(crate) fn propagate(
    persistent: &mut (impl GetKnowledgeBase + PutKnowledgeBase),
    loaded: &mut impl Loaded,
    original_term: &FatTerm,
    arg_changes: &[ArgsChange],
    updated_term: &FatTerm,
    affected: &[String],
) {
    let term_name = original_term.meta.term.name.clone();
    debug!("Direct change propagation");
    let mut affected_terms = changes::propagation::apply(
        original_term,
        arg_changes,
        updated_term,
        // TODO: reuse the trait impl from with_confirmation
        persistent,
    );
    affected_terms.insert(term_name.clone(), updated_term.to_owned());

    for (affected_term_name, affected_updated_term) in affected_terms.into_iter() {
        persistent
            .put(&affected_term_name, affected_updated_term)
            .expect("writing to persistence layer should not fail");
    }

    let update_fn = |in_term: &FatTerm| -> FatTerm {
        changes::propagation::apply(
            original_term,
            arg_changes,
            updated_term,
            &SingleTerm {
                term: in_term.to_owned(),
            },
        )
        .get(&in_term.meta.term.name)
        .unwrap()
        .to_owned()
    };

    loaded.update_with(&term_name, |_| updated_term.clone());
    for affected_term_name in affected {
        loaded.update_with(&affected_term_name, update_fn);
    }
}

pub(crate) fn propagate_deletion(
    persistent: &mut (impl GetKnowledgeBase + PutKnowledgeBase),
    loaded: &mut impl Loaded,
    term: &FatTerm,
) {
    debug!("Direct delete propagation");
    let affected_terms = changes::propagation::apply_deletion(term, persistent);

    for (affected_term_name, affected_updated_term) in affected_terms.into_iter() {
        persistent
            .put(&affected_term_name, affected_updated_term)
            .expect("writing to persistence layer should not fail");
    }

    let update_fn = |in_term: &FatTerm| -> FatTerm {
        changes::propagation::apply_deletion(
            term,
            &SingleTerm {
                term: in_term.to_owned(),
            },
        )
        .get(&in_term.meta.term.name)
        .unwrap()
        .to_owned()
    };

    for affected_term_name in changes::propagation::affected_from_deletion(term) {
        loaded.update_with(&affected_term_name, update_fn);
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

impl GetKnowledgeBase for SingleTerm {
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        if term_name == self.term.meta.term.name {
            Some(self.term.clone())
        } else {
            None
        }
    }
}
