use std::collections::HashMap;

use its_logical::changes::change::{Apply, Change};
use its_logical::knowledge::model::fat_term::FatTerm;
use its_logical::knowledge::store::{Get, Put};

use its_logical::changes::deletion::Deletion;
mod loaded;

pub(crate) fn propagate(
    persistent: &mut (impl Get + Put),
    loaded: &mut impl loaded::Loaded,
    change: &Change,
) {
    let term_name = change.original_name();
    let updated_term = change.changed().to_owned();

    let update_fn = |in_term: &FatTerm| -> FatTerm {
        in_term
            .apply(&change)
            .get(&in_term.meta.term.name)
            // the change might not affect the in_term so it needs to be returned as is
            .unwrap_or(in_term)
            .to_owned()
    };
    let mut affected_terms = persistent.apply(&change);

    loaded.update_with(&term_name, |_| updated_term.clone());
    affected_terms.insert(term_name.clone(), updated_term);

    apply_updates(persistent, loaded, affected_terms, update_fn);
}

// propagate_deletion doesn't delete the persistent term, nor does something for its loaded state
pub(crate) fn propagate_deletion(
    persistent: &mut (impl Get + Put),
    loaded: &mut impl loaded::Loaded,
    term: &FatTerm,
) {
    let update_fn = |in_term: &FatTerm| -> FatTerm {
        term.apply_deletion(in_term)
            .get(&in_term.meta.term.name)
            // the deletion might not affect the in_term so it needs to be returned as is
            .unwrap_or(in_term)
            .to_owned()
    };

    let affected_terms = term.apply_deletion(persistent);
    apply_updates(persistent, loaded, affected_terms, update_fn);
}

fn apply_updates(
    persistent: &mut (impl Get + Put),
    loaded: &mut impl loaded::Loaded,
    updates: HashMap<String, FatTerm>,
    update_fn: impl Fn(&FatTerm) -> FatTerm,
) {
    for affected_term_name in updates.keys() {
        loaded.update_with(&affected_term_name, &update_fn);
    }

    for (affected_term_name, affected_updated_term) in updates.into_iter() {
        persistent
            .put(&affected_term_name, affected_updated_term)
            .expect("writing to persistence layer should not fail");
    }
}
