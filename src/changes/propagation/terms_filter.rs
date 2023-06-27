use std::collections::HashMap;

use crate::model::fat_term::FatTerm;

use super::Terms;

pub(crate) trait TermsFilter {
    fn get<'a>(&'a mut self, name: &str) -> Option<&'a mut FatTerm>;
    fn put(&mut self, name: &str, term: &FatTerm);
    fn all_terms(&self) -> HashMap<String, FatTerm>;
}

// will only ever know about a single term
pub(crate) fn with_single_term(term: &FatTerm) -> impl TermsFilter {
    SingleTerm::new(term)
}

// will try to use terms available through the provided Terms implementation
pub(crate) fn with_terms_cache<'a, T: Terms>(terms: &'a T) -> impl TermsFilter + 'a {
    TermsCache::new(terms)
}

struct SingleTerm {
    term: FatTerm,
}

impl SingleTerm {
    fn new(term: &FatTerm) -> Self {
        Self {
            term: term.to_owned(),
        }
    }
}

impl TermsFilter for SingleTerm {
    fn get<'a>(&'a mut self, name: &str) -> Option<&'a mut FatTerm> {
        if name != self.term.meta.term.name {
            return None;
        }
        Some(&mut self.term)
    }

    fn all_terms(&self) -> HashMap<String, FatTerm> {
        HashMap::from([(self.term.meta.term.name.clone(), self.term.clone())])
    }

    fn put(&mut self, name: &str, term: &FatTerm) {
        if name == self.term.meta.term.name {
            self.term = term.to_owned();
        }
    }
}

struct TermsCache<'a, T: Terms> {
    updated_terms: HashMap<String, FatTerm>,
    terms: &'a T,
}

impl<'a, T: Terms> TermsCache<'a, T> {
    fn new(terms: &'a T) -> Self {
        Self {
            updated_terms: HashMap::new(),
            terms,
        }
    }
}

impl<'a, T: Terms> TermsFilter for TermsCache<'a, T> {
    fn get<'b>(&'b mut self, name: &str) -> Option<&'b mut FatTerm> {
        Some(
            self.updated_terms
                .entry(name.to_string())
                .or_insert(self.terms.get(name).unwrap()),
        )
    }

    fn all_terms(&self) -> HashMap<String, FatTerm> {
        self.updated_terms.clone()
    }

    fn put(&mut self, name: &str, term: &FatTerm) {
        self.updated_terms.insert(name.to_string(), term.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::fat_term::FatTerm;

    struct MockTerms {
        terms: HashMap<String, FatTerm>,
    }

    impl Terms for MockTerms {
        fn get(&self, term_name: &str) -> Option<FatTerm> {
            self.terms.get(term_name).cloned()
        }
    }

    #[test]
    fn test_single_term() {
        let term_name = "The_Imporant_Term".to_string();
        let term_unnamed = "The_unimporant_Term".to_string();

        let mut term1 = FatTerm::default();
        term1.meta.term.name = term_name.clone();

        let mut term2 = FatTerm::default();
        term2.meta.term.name = term_unnamed.clone();

        let mut single_term = SingleTerm::new(&term1);

        assert_eq!(single_term.get(&term_name), Some(&mut term1));
        assert_eq!(single_term.get(&term_unnamed), None);

        let all_terms = single_term.all_terms();
        assert_eq!(all_terms.len(), 1);
        assert_eq!(all_terms[&term_name], term1);
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

        // but then lets put the initial version back in
        terms_cache.put(&term1_name, &term1);
        assert_eq!(terms_cache.get(&term1_name), Some(&mut term1));

        let all_terms = terms_cache.all_terms();
        assert_eq!(all_terms.len(), 2);
        assert_eq!(all_terms[&term1_name], term1);
        assert_eq!(all_terms[&term2_name], term2);
    }
}
