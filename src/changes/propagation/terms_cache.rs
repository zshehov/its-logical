use crate::knowledge::model::fat_term::FatTerm;
use crate::knowledge::store::Get;
use std::collections::HashMap;

pub(crate) struct TermsCache<'a, T: Get> {
    updated_terms: HashMap<String, FatTerm>,
    terms: &'a T,
}

impl<'a, T: Get> TermsCache<'a, T> {
    pub(crate) fn new(terms: &'a T) -> Self {
        Self {
            updated_terms: HashMap::new(),
            terms,
        }
    }
    pub(crate) fn get<'b>(&'b mut self, name: &str) -> Option<&'b mut FatTerm> {
        match self.updated_terms.entry(name.to_string()) {
            std::collections::hash_map::Entry::Occupied(e) => Some(e.into_mut()),
            std::collections::hash_map::Entry::Vacant(e) => match self.terms.get(name) {
                Some(term) => Some(e.insert(term)),
                None => None,
            },
        }
    }

    pub(crate) fn all_terms(self) -> HashMap<String, FatTerm> {
        self.updated_terms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTerms {
        terms: HashMap<String, FatTerm>,
    }

    impl Get for MockTerms {
        fn get(&self, term_name: &str) -> Option<FatTerm> {
            self.terms.get(term_name).cloned()
        }
    }

    #[test]
    fn test_terms_cache() {
        let term1_name = "term1".to_string();
        let mut term1 = FatTerm::default();
        term1.meta.term.name = term1_name.clone();

        let term2_name = "term2".to_string();
        let mut term2 = FatTerm::default();
        term2.meta.term.name = term2_name.clone();

        let mock_terms = MockTerms {
            terms: HashMap::from([
                (term1_name.clone(), term1.clone()),
                (term2_name.clone(), term2.clone()),
            ]),
        };

        let mut terms_cache = TermsCache::new(&mock_terms);

        assert_eq!(terms_cache.get(&term1_name), Some(&mut term1));
        assert_eq!(terms_cache.get(&term2_name), Some(&mut term2));

        terms_cache.get(&term1_name).unwrap().meta.term.desc = "some description".to_string();
        assert_eq!(
            terms_cache.get(&term1_name).unwrap().meta.term.desc,
            "some description".to_string()
        );
    }
}
