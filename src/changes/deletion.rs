use std::collections::HashMap;

use crate::knowledge::{self, model::fat_term::FatTerm};

use super::terms_cache::TermsCache;

pub trait Deletion {
    fn deletion_affects(&self) -> Vec<String>;
    fn apply_deletion(&self, terms: &impl knowledge::store::Get) -> HashMap<String, FatTerm>;
}

impl Deletion for FatTerm {
    fn deletion_affects(&self) -> Vec<String> {
        let mut affected_terms = vec![];
        // need to remove the term from all the terms' "referred by" field
        affected_terms.append(&mut self.mentioned_terms().into_iter().collect());
        // need to remove the term from all the terms' rules that refer to it
        affected_terms.append(&mut self.meta.referred_by.clone());
        affected_terms
    }

    fn apply_deletion(&self, terms: &impl knowledge::store::Get) -> HashMap<String, FatTerm> {
        let mut terms_cache = TermsCache::new(terms);
        for rule in self.term.rules.iter() {
            for body_term in &rule.body {
                if let Some(term) = terms_cache.get(&body_term.name) {
                    term.remove_referred_by(&self.meta.term.name);
                }
            }
        }

        for referred_by_term_name in &self.meta.referred_by {
            if let Some(term) = terms_cache.get(referred_by_term_name) {
                for rule in &mut term.term.rules {
                    rule.body
                        .retain(|body_term| body_term.name != self.meta.term.name);
                }
                term.term.rules.retain(|rule| !rule.body.is_empty());
            }
        }
        terms_cache.all_terms()
    }
}