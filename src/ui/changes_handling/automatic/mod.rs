use crate::knowledge::model::fat_term::FatTerm;
use crate::knowledge::store::{Get, Put};

use crate::changes::{self, Deletion};

mod loaded;

impl Get for FatTerm {
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        if term_name == self.meta.term.name {
            Some(self.clone())
        } else {
            None
        }
    }
}

pub(crate) fn propagate(
    persistent: &mut (impl Get + Put),
    loaded: &mut impl loaded::Loaded,
    original_term: &FatTerm,
    updated_term: &FatTerm,
    affected: &[String],
) {
    let change = changes::Change::new(original_term.to_owned(), &[], updated_term.to_owned());
    let term_name = original_term.meta.term.name.clone();
    let mut affected_terms = change.apply(persistent);

    affected_terms.insert(term_name.clone(), updated_term.to_owned());

    for (affected_term_name, affected_updated_term) in affected_terms.into_iter() {
        persistent
            .put(&affected_term_name, affected_updated_term)
            .expect("writing to persistence layer should not fail");
    }

    let update_fn = |in_term: &FatTerm| -> FatTerm {
        change
            .apply(in_term)
            .get(&in_term.meta.term.name)
            .unwrap()
            .to_owned()
    };

    loaded.update_with(&term_name, |_| updated_term.clone());
    for affected_term_name in affected {
        loaded.update_with(affected_term_name, update_fn);
    }
}

pub(crate) fn propagate_deletion(
    persistent: &mut (impl Get + Put),
    loaded: &mut impl loaded::Loaded,
    term: &FatTerm,
) {
    let affected_terms = term.apply_deletion(persistent);

    for (affected_term_name, affected_updated_term) in affected_terms.into_iter() {
        persistent
            .put(&affected_term_name, affected_updated_term)
            .expect("writing to persistence layer should not fail");
    }

    let update_fn = |in_term: &FatTerm| -> FatTerm {
        term.apply_deletion(in_term)
            .get(&in_term.meta.term.name)
            .unwrap()
            .to_owned()
    };

    for affected_term_name in term.deletion_affects() {
        loaded.update_with(&affected_term_name, update_fn);
    }
}
