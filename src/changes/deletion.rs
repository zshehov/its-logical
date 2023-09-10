use std::collections::HashMap;

use crate::knowledge::{self, model::fat_term::FatTerm};

use super::terms_cache::TermsCache;

pub trait Deletion {
    fn apply_deletion(&self, terms: &impl knowledge::store::Get) -> HashMap<String, FatTerm>;
    fn affects(&self) -> &[String];
}

impl Deletion for FatTerm {
    fn affects(&self) -> &[String] {
        &self.meta.referred_by
    }

    fn apply_deletion(&self, terms: &impl knowledge::store::Get) -> HashMap<String, FatTerm> {
        let mut terms_cache = TermsCache::new(terms);
        for mentioned_term_name in self.mentioned_terms().into_iter() {
            if let Some(term) = terms_cache.get(&mentioned_term_name) {
                term.remove_referred_by(&self.meta.term.name);
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
