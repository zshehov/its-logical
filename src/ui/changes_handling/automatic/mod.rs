use tracing::debug;

use crate::{
    changes::{self, ArgsChange},
    model::fat_term::FatTerm,
    term_knowledge_base::{GetKnowledgeBase, PutKnowledgeBase},
};

mod loaded;

impl GetKnowledgeBase for FatTerm {
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        if term_name == self.meta.term.name {
            Some(self.clone())
        } else {
            None
        }
    }
}

pub(crate) fn propagate(
    persistent: &mut (impl GetKnowledgeBase + PutKnowledgeBase),
    loaded: &mut impl loaded::Loaded,
    original_term: &FatTerm,
    updated_term: &FatTerm,
    affected: &[String],
) {
    let term_name = original_term.meta.term.name.clone();
    debug!("Direct change propagation");
    let mut affected_terms =
        changes::propagation::apply(original_term, &[], updated_term, persistent);
    affected_terms.insert(term_name.clone(), updated_term.to_owned());

    for (affected_term_name, affected_updated_term) in affected_terms.into_iter() {
        persistent
            .put(&affected_term_name, affected_updated_term)
            .expect("writing to persistence layer should not fail");
    }

    let update_fn = |in_term: &FatTerm| -> FatTerm {
        changes::propagation::apply(original_term, &[], updated_term, in_term)
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
    loaded: &mut impl loaded::Loaded,
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
        changes::propagation::apply_deletion(term, in_term)
            .get(&in_term.meta.term.name)
            .unwrap()
            .to_owned()
    };

    for affected_term_name in changes::propagation::affected_from_deletion(term) {
        loaded.update_with(&affected_term_name, update_fn);
    }
}
