use std::collections::HashMap;

use crate::knowledge::{
    self,
    model::{
        comment::name_description::NameDescription, fat_term::FatTerm,
        term::args_binding::ArgsBinding,
    },
};

use super::terms_cache::TermsCache;

pub struct Change {
    original: FatTerm,
    args_changes: Vec<ArgsChange>,
    changed: FatTerm,
}

#[derive(Clone, Debug)]
pub enum ArgsChange {
    Pushed(NameDescription),
    Moved(Vec<usize>),
    Removed(usize),
}

impl ArgsChange {
    pub fn apply(&self, binding: &mut ArgsBinding) -> Option<String> {
        match self {
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
}

pub trait Apply {
    fn apply(&self, change: &Change) -> HashMap<String, FatTerm>;
}

impl<T: knowledge::store::Get> Apply for T {
    fn apply(&self, change: &Change) -> HashMap<String, FatTerm> {
        let mut terms_cache = TermsCache::new(self);

        if !change.args_changes.is_empty() {
            for referred_by_term_name in &change.changed.meta.referred_by {
                if let Some(term) = terms_cache.get(referred_by_term_name) {
                    apply_args_changes(&change, term);
                }
            }
        }

        let (new, removed) = changes_in_mentioned_terms(&change);
        for term_name_with_removed_mention in &removed {
            if let Some(term) = terms_cache.get(term_name_with_removed_mention) {
                term.remove_referred_by(&change.original.meta.term.name);
            }
        }

        for term_name_with_new_mention in &new {
            if let Some(term) = terms_cache.get(term_name_with_new_mention) {
                term.add_referred_by(&change.original.meta.term.name);
            }
        }
        // once all externally propagated changes are applied with the original name,
        // the potential name change is addressed
        if &change.original.meta.term.name != &change.changed.meta.term.name {
            for rule in change.changed.term.rules.iter() {
                for body_term in &rule.body {
                    if let Some(term) = terms_cache.get(&body_term.name) {
                        term.rename_referred_by(
                            &change.original.meta.term.name,
                            &change.changed.meta.term.name,
                        );
                    }
                }
            }

            for referred_by_term_name in &change.changed.meta.referred_by {
                if let Some(term) = terms_cache.get(referred_by_term_name) {
                    for rule in &mut term.term.rules {
                        for body_term in &mut rule.body {
                            if &body_term.name == &change.original.meta.term.name {
                                body_term.name = change.changed.meta.term.name.clone();
                            }
                        }
                    }
                }
            }
        }
        terms_cache.all_terms()
    }
}

// enable applying a change on a single term
impl knowledge::store::Get for FatTerm {
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        if term_name == self.meta.term.name {
            Some(self.clone())
        } else {
            None
        }
    }
}

impl Change {
    pub fn new(original: FatTerm, args_changes: &[ArgsChange], changed: FatTerm) -> Self {
        Self {
            original,
            args_changes: args_changes.to_vec(),
            changed,
        }
    }

    pub fn original(&self) -> &FatTerm {
        &self.original
    }

    pub fn changed(&self) -> &FatTerm {
        &self.changed
    }

    pub fn arg_changes(&self) -> &[ArgsChange] {
        &self.args_changes
    }

    // returned "mentioned" terms are ones that actually are changed (newly mentioned/not mentioned
    // any longer or in case of name change each mentioned term will have its "referred by" field
    // changed)
    pub fn affects(&self) -> (Vec<String>, Vec<String>) {
        let mut mentioned = vec![];
        let mut referred_by = vec![];
        let mut include_referred_by = false;
        let mut include_mentioned = false;

        if &self.original.meta.term.name != &self.changed.meta.term.name {
            include_referred_by = true;
            include_mentioned = true;
        }

        if include_mentioned {
            // all mentioned are already included so there's no need to figure out
            // new and old
        } else {
            let (mut new, mut removed) = changes_in_mentioned_terms(self);

            mentioned.append(&mut new);
            mentioned.append(&mut removed);
        }

        if !self.args_changes.is_empty() {
            include_referred_by = true;
        }

        if include_referred_by {
            referred_by.append(&mut self.changed.meta.referred_by.clone());
        }
        if include_mentioned {
            let old_mentioned = self.original.mentioned_terms();
            let current_mentioned = self.changed.mentioned_terms();

            mentioned.append(&mut old_mentioned.union(&current_mentioned).cloned().collect());
        }
        (mentioned, referred_by)
    }
}

fn changes_in_mentioned_terms(change: &Change) -> (Vec<String>, Vec<String>) {
    let old_related_terms = change.original.mentioned_terms();
    let related_terms = change.changed.mentioned_terms();

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

fn apply_args_changes(change: &Change, target_term: &mut FatTerm) {
    for rule in &mut target_term.term.rules {
        for body_term in &mut rule.body {
            if &body_term.name == &change.original.meta.term.name {
                for change in &change.args_changes {
                    change.apply(&mut body_term.arg_bindings);
                }
            }
        }
    }
}
