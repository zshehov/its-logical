use crate::knowledge::model::{fat_term::FatTerm, term::args_binding::ArgsBinding};
use crate::knowledge::store::Get;
use std::collections::HashMap;

use self::terms_cache::TermsCache;

use super::ArgsChange;

mod terms_cache;

// returned "mentioned" terms are ones that actually are changed (newly mentioned/not mentioned
// any longer or in case of name change each mentioned term will have its "referred by" field
// changed)
pub(crate) fn affected_from_changes(
    original: &FatTerm,
    updated: &FatTerm,
    args_changes: &[ArgsChange],
) -> (Vec<String>, Vec<String>) {
    let mut mentioned = vec![];
    let mut referred_by = vec![];
    let mut include_referred_by = false;
    let mut include_mentioned = false;

    if original.meta.term.name != updated.meta.term.name {
        include_referred_by = true;
        include_mentioned = true;
    }

    if include_mentioned {
        // all mentioned are already included so there's no need to figure out
        // new and old
    } else {
        let (mut new, mut removed) = changes_in_mentioned_terms(original, updated);

        mentioned.append(&mut new);
        mentioned.append(&mut removed);
    }

    if !args_changes.is_empty() {
        include_referred_by = true;
    }

    if include_referred_by {
        referred_by.append(&mut updated.meta.referred_by.clone());
    }
    if include_mentioned {
        let old_mentioned = original.mentioned_terms();
        let current_mentioned = updated.mentioned_terms();

        mentioned.append(&mut old_mentioned.union(&current_mentioned).cloned().collect());
    }
    (mentioned, referred_by)
}

pub(crate) fn affected_from_deletion(original: &FatTerm) -> Vec<String> {
    let mut affected_terms = vec![];
    // need to remove the term from all the terms' "referred by" field
    affected_terms.append(&mut original.mentioned_terms().into_iter().collect());
    // need to remove the term from all the terms' rules that refer to it
    affected_terms.append(&mut original.meta.referred_by.clone());
    affected_terms
}

pub(crate) fn apply(
    original: &FatTerm,
    args_changes: &[ArgsChange],
    updated: &FatTerm,
    terms: &impl Get,
) -> HashMap<String, FatTerm> {
    let mut terms_cache = TermsCache::new(terms);

    if !args_changes.is_empty() {
        for referred_by_term_name in &updated.meta.referred_by {
            if let Some(term) = terms_cache.get(referred_by_term_name) {
                apply_args_changes(term, &original.meta.term.name, args_changes);
            }
        }
    }

    let (new, removed) = changes_in_mentioned_terms(original, updated);
    for term_name_with_removed_mention in &removed {
        if let Some(term) = terms_cache.get(term_name_with_removed_mention) {
            term.remove_referred_by(&original.meta.term.name);
        }
    }

    for term_name_with_new_mention in &new {
        if let Some(term) = terms_cache.get(term_name_with_new_mention) {
            term.add_referred_by(&original.meta.term.name);
        }
    }
    // once all externally propagated changes are applied with the original name,
    // the potential name change is addressed
    if original.meta.term.name != updated.meta.term.name {
        for rule in updated.term.rules.iter() {
            for body_term in &rule.body {
                if let Some(term) = terms_cache.get(&body_term.name) {
                    term.rename_referred_by(&original.meta.term.name, &updated.meta.term.name);
                }
            }
        }

        for referred_by_term_name in &updated.meta.referred_by {
            if let Some(term) = terms_cache.get(referred_by_term_name) {
                for rule in &mut term.term.rules {
                    for body_term in &mut rule.body {
                        if body_term.name == original.meta.term.name {
                            body_term.name = updated.meta.term.name.clone();
                        }
                    }
                }
            }
        }
    }
    terms_cache.all_terms()
}

pub(crate) fn apply_deletion(deleted_term: &FatTerm, terms: &impl Get) -> HashMap<String, FatTerm> {
    let mut terms_cache = TermsCache::new(terms);
    for rule in deleted_term.term.rules.iter() {
        for body_term in &rule.body {
            if let Some(term) = terms_cache.get(&body_term.name) {
                term.remove_referred_by(&deleted_term.meta.term.name);
            }
        }
    }

    for referred_by_term_name in &deleted_term.meta.referred_by {
        if let Some(term) = terms_cache.get(referred_by_term_name) {
            for rule in &mut term.term.rules {
                rule.body
                    .retain(|body_term| body_term.name != deleted_term.meta.term.name);
            }
            term.term.rules.retain(|rule| !rule.body.is_empty());
        }
    }
    terms_cache.all_terms()
}

fn changes_in_mentioned_terms(
    original_term: &FatTerm,
    term: &FatTerm,
) -> (Vec<String>, Vec<String>) {
    let old_related_terms = original_term.mentioned_terms();
    let related_terms = term.mentioned_terms();

    return (
        related_terms
            .difference(&old_related_terms)
            .cloned()
            .collect(),
        old_related_terms
            .difference(&related_terms)
            .cloned()
            .collect(),
    );
}

fn apply_args_changes<'a>(term: &mut FatTerm, term_with_arg_change: &str, changes: &[ArgsChange]) {
    for rule in &mut term.term.rules {
        for body_term in &mut rule.body {
            if body_term.name == term_with_arg_change {
                for change in changes {
                    apply_binding_change(change, &mut body_term.arg_bindings);
                }
            }
        }
    }
}

pub(crate) fn apply_binding_change(
    change: &ArgsChange,
    binding: &mut ArgsBinding,
) -> Option<String> {
    match change {
        ArgsChange::Pushed(_) => binding.binding.push("_".to_string()),
        ArgsChange::Moved(moves) => {
            let mut projected = vec!["".to_string(); binding.binding.len()];

            for (idx, old_idx) in moves.iter().enumerate() {
                projected[idx] = binding.binding[*old_idx].clone();
            }
            binding.binding = projected;
        }
        ArgsChange::Removed(removed_idx) => return Some(binding.binding.remove(*removed_idx)),
    }
    None
}

