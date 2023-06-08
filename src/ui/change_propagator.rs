use std::collections::{HashMap, HashSet};

use tracing::debug;

use crate::{
    model::{comment::name_description::NameDescription, fat_term::FatTerm},
    term_knowledge_base::TermsKnowledgeBase,
};

use super::widgets::{
    drag_and_drop,
    term_screen::{Change, Result},
};

pub(crate) fn apply_changes<T: TermsKnowledgeBase>(
    changes: &Result,
    terms: &T,
) -> (HashMap<String, FatTerm>, bool) {
    let mut updated_terms = HashMap::new();
    let mut needs_confirmation = false;

    match changes {
        Result::Changes(changes, original_name, updated_term) => {
            let original_name = original_name.to_owned();
            for change in changes {
                match change {
                    Change::DescriptionChange | Change::FactsChange | Change::ArgRename => {
                        debug!("internal changes");
                    }
                    Change::ArgChanges(arg_changes) => {
                        for referred_by_term_name in &updated_term.meta.referred_by {
                            let referred_by_term = updated_terms
                                .entry(referred_by_term_name.clone())
                                .or_insert(terms.get(&referred_by_term_name).unwrap());

                            apply_arg_changes(referred_by_term, &original_name, arg_changes.iter());
                        }
                        if updated_term.meta.referred_by.len() > 0 {
                            for arg_change in arg_changes {
                                if let drag_and_drop::Change::Pushed(_)
                                | drag_and_drop::Change::Removed(_, _) = arg_change
                                {
                                    // currently only a new or removed argument triggers user
                                    // intervention - all other changes can be applied
                                    // automaticallly
                                    needs_confirmation = true;
                                }
                            }
                        }
                    }
                    Change::RuleChanges(_) => {
                        let (new, removed) = changes_in_mentioned_terms(
                            &terms.get(&original_name).unwrap(),
                            &updated_term,
                        );
                        for term_name_with_removed_mention in removed {
                            let term_with_removed_mention = updated_terms
                                .entry(term_name_with_removed_mention.clone())
                                .or_insert(terms.get(&term_name_with_removed_mention).unwrap());
                            term_with_removed_mention.remove_referred_by(&original_name);
                        }

                        for term_name_with_new_mention in new {
                            let term_with_new_mention = updated_terms
                                .entry(term_name_with_new_mention.to_owned())
                                .or_insert(terms.get(&term_name_with_new_mention).unwrap());
                            term_with_new_mention.add_referred_by(&original_name);
                        }
                    }
                };
            }
            // once all externally propagated changes are applied with the original name,
            // the potential name change is addressed
            if original_name != updated_term.meta.term.name {
                updated_terms
                    .entry(original_name.clone())
                    .or_insert(updated_term.clone());

                for rule in updated_term.term.rules.iter() {
                    for body_term in &rule.body {
                        let mentioned_term = updated_terms
                            .entry(body_term.name.clone())
                            .or_insert(terms.get(&body_term.name).unwrap());
                        mentioned_term
                            .rename_referred_by(&original_name, &updated_term.meta.term.name);
                    }
                }

                for referred_by_term_name in &updated_term.meta.referred_by {
                    let referred_by_term = updated_terms
                        .entry(referred_by_term_name.to_owned())
                        .or_insert(terms.get(&referred_by_term_name).unwrap());

                    for rule in &mut referred_by_term.term.rules {
                        for body_term in &mut rule.body {
                            if body_term.name == original_name {
                                body_term.name = updated_term.meta.term.name.clone();
                            }
                        }
                    }
                }
            }
            // any change also triggers the internal change
            updated_terms
                .entry(original_name.clone())
                .or_insert(updated_term.clone());
        }
        Result::Deleted(term_name) => {
            let original_term = terms.get(term_name).unwrap();

            for rule in original_term.term.rules.iter() {
                for body_term in &rule.body {
                    let mentioned_term = updated_terms
                        .entry(body_term.name.clone())
                        .or_insert(terms.get(&body_term.name).unwrap());
                    mentioned_term.remove_referred_by(term_name);
                }
            }

            for referred_by_term_name in &original_term.meta.referred_by {
                let referred_by_term = updated_terms
                    .entry(referred_by_term_name.to_owned())
                    .or_insert(terms.get(&referred_by_term_name).unwrap());

                for rule in &mut referred_by_term.term.rules {
                    let before_removal_body_term_count = rule.body.len();
                    rule.body.retain(|body_term| body_term.name != *term_name);

                    if rule.body.len() < before_removal_body_term_count {
                        // A confirmation is needed only if actual removing was done. There is the
                        // case when the user has already confirmed the deletion and this code path
                        // does not remove anything.
                        needs_confirmation = true;
                    }
                }
            }
        }
    }
    (updated_terms, needs_confirmation)
}

fn changes_in_mentioned_terms(
    original_term: &FatTerm,
    term: &FatTerm,
) -> (Vec<String>, Vec<String>) {
    let mut related_terms = HashSet::<String>::new();
    let mut old_related_terms = HashSet::<String>::new();

    // grab all the old related terms
    for rule in original_term.term.rules.iter() {
        for body_term in &rule.body {
            old_related_terms.insert(body_term.name.clone());
        }
    }
    // grab all the currently related terms
    for rule in term.term.rules.iter() {
        for body_term in &rule.body {
            related_terms.insert(body_term.name.clone());
        }
    }

    return (
        related_terms
            .difference(&old_related_terms)
            .into_iter()
            .cloned()
            .collect(),
        old_related_terms
            .difference(&related_terms)
            .into_iter()
            .cloned()
            .collect(),
    );
}

fn apply_arg_changes<'a>(
    term: &mut FatTerm,
    term_with_arg_change: &str,
    changes: impl Iterator<Item = &'a drag_and_drop::Change<NameDescription>>,
) {
    for change in changes {
        for rule in &mut term.term.rules {
            for body_term in &mut rule.body {
                if body_term.name == term_with_arg_change {
                    match change {
                        drag_and_drop::Change::Pushed(_) => {
                            body_term.arg_bindings.binding.push("_".to_string());
                        }
                        drag_and_drop::Change::Moved(moves) => {
                            let mut projected =
                                vec!["".to_string(); body_term.arg_bindings.binding.len()];

                            for (idx, old_idx) in moves.iter().enumerate() {
                                projected[idx] = body_term.arg_bindings.binding[*old_idx].clone();
                            }
                            body_term.arg_bindings.binding = projected;
                        }
                        drag_and_drop::Change::Removed(removed_idx, _) => {
                            body_term.arg_bindings.binding.remove(*removed_idx);
                        }
                    }
                }
            }
        }
    }
}
