use std::collections::HashSet;

use its_logical::{
    changes::{
        change::{Apply, Change},
        deletion::Deletion,
    },
    knowledge::{self, model::fat_term::FatTerm},
};
use tracing::debug;

use crate::terms_cache::{
    change_handling::{automatic, with_confirmation},
    NamedTerm, TermHolder, TermsCache, TwoPhaseTerm,
};

pub(crate) fn propagate_change<T, K>(
    change: &Change,
    store: &mut (impl knowledge::store::Get + knowledge::store::Put),
    cache: &mut TermsCache<T, K>,
) where
    T: NamedTerm + automatic::Apply,
    K: TwoPhaseTerm<Creator = T> + automatic::Apply + with_confirmation::Apply,
{
    let (mut mentioned, referred_by) = change.affects();

    mentioned.extend(referred_by.clone());
    let all_affected: Vec<String> = mentioned.into_iter().collect();

    debug!(
        "Changes made for {}. Propagating to: {:?}",
        change.original().meta.term.name,
        all_affected
    );

    let is_automatic = /* the changes are not worthy of user confirmation */
    change.arg_changes().is_empty()
        || /* no referring term is affected */ referred_by.is_empty();

    let change_source_in_commit = cache
        .get(&change.original().meta.term.name)
        .map(|t| matches!(t, TermHolder::TwoPhase(_)))
        .unwrap_or(false);

    if is_automatic && !change_source_in_commit {
        debug!("automatic propagation");
        cache.apply_automatic_change(change);

        let mut changes_for_store = store.apply(change);
        changes_for_store.insert(
            change.original().meta.term.name.clone(),
            change.changed().to_owned(),
        );

        for (term_name, with_applied_change) in changes_for_store {
            store
                .put(&term_name, with_applied_change)
                .expect("persistence layer changes should not fail");
        }
    } else {
        debug!("2 phase commit propagation");
        cache.apply_for_confirmation_change(store, change);
    }
    // if there is an ongoing 2phase commit among one of `updated_term`'s newly mentioned terms,
    // all the changes in the commit need to be applied on `updated_term`
    if cache.iter().any(|t| matches!(t, TermHolder::TwoPhase(_))) {
        cache.repeat_ongoing_commit_changes(change, is_automatic);
    }
}

pub(crate) fn finish_commit<T, K>(
    store: &mut (impl knowledge::store::Get + knowledge::store::Put + knowledge::store::Delete),
    cache: &mut TermsCache<T, K>,
) -> HashSet<String>
where
    T: NamedTerm,
    K: TwoPhaseTerm<Creator = T>,
{
    let changed_terms = cache.finish_commit();
    let mut deleted = HashSet::new();
    for (changed_term_original_name, change) in changed_terms {
        match change {
            crate::terms_cache::change_handling::FinishedCommitResult::Changed(changed_term) => {
                store.put(&changed_term_original_name, changed_term);
            }
            crate::terms_cache::change_handling::FinishedCommitResult::Deleted => {
                store.delete(&changed_term_original_name);
                deleted.insert(changed_term_original_name);
            }
        }
    }
    deleted
}

pub(crate) fn revert_commit<T, K>(cache: &mut TermsCache<T, K>)
where
    T: NamedTerm,
    K: TwoPhaseTerm<Creator = T>,
{
    cache.revert_commit();
}

pub(crate) fn propagate_deletion<T, K>(
    term: &FatTerm,
    store: &mut (impl knowledge::store::Get + knowledge::store::Put + knowledge::store::Delete),
    cache: &mut TermsCache<T, K>,
) -> bool
where
    T: NamedTerm + automatic::Apply,
    K: TwoPhaseTerm<Creator = T> + automatic::Apply + with_confirmation::Apply,
{
    if term.meta.referred_by.is_empty() {
        debug!("automatic deletion");
        cache.apply_automatic_deletion(term);
        for (term_name, with_applied_deletion) in term.apply_deletion(store) {
            store
                .put(&term_name, with_applied_deletion)
                .expect("persistence layer changes should not fail");
        }
        store.delete(&term.meta.term.name);
        true
    } else {
        debug!("deletion with confirmation");
        cache.apply_for_confirmation_delete(term, store);
        false
    }
}
