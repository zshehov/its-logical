use std::collections::HashMap;
use std::path::Path;

use crate::knowledge::model::fat_term::FatTerm;
use crate::knowledge::model::term::bound_term::BoundTerm;
use crate::knowledge::store::{Consult, Delete, Get, Keys, Load, Put, TermsStore};

pub struct InMemoryTerms {
    map: HashMap<String, FatTerm>,
    vec: Vec<String>,
}

impl InMemoryTerms {
    pub fn new(map: HashMap<String, FatTerm>) -> Self {
        let vec = map.keys().cloned().collect();
        Self { map, vec }
    }
}

impl Get for InMemoryTerms {
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        self.map.get(term_name).cloned()
    }
}

impl Put for InMemoryTerms {
    fn put(&mut self, term_name: &str, term: FatTerm) {
        if self.map.contains_key(term_name) {
            self.map.remove(term_name);
        }
        self.map.insert(term.meta.term.name.clone(), term);
    }
}

impl Delete for InMemoryTerms {
    fn delete(&mut self, term_name: &str) {
        self.map.remove(term_name);
        let pos = self.vec.iter().position(|t| t == term_name).unwrap();
        self.vec.swap_remove(pos);
    }
}

impl Keys for InMemoryTerms {
    fn keys(&self) -> &Vec<String> {
        &self.vec
    }
}

impl Load for InMemoryTerms {
    fn load(_path: &Path) -> InMemoryTerms {
        todo!()
    }

    type Store = InMemoryTerms;
}

impl Consult for InMemoryTerms {
    fn consult(&mut self, _: &BoundTerm) -> Vec<HashMap<String, String>> {
        todo!()
    }
}

impl TermsStore for InMemoryTerms {}
