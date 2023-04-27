use std::collections::HashMap;

use crate::model::fat_term::FatTerm;

pub enum KnowledgeBaseError {
    NotFound,
    AlreadyPresent,
    //TODO:  InvalidTerm,
}

pub trait TermsKnowledgeBase {
    fn get(&self, term_name: &str) -> Option<&FatTerm>;
    fn edit(&mut self, term_name: &str, updated: &FatTerm) -> Result<(), KnowledgeBaseError>;
    fn put(&mut self, term_name: &str, term: FatTerm) -> Result<(), KnowledgeBaseError>;
    fn keys(&self) -> &Vec<String>;
    fn delete(&mut self, term_name: &str);
}

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

impl TermsKnowledgeBase for InMemoryTerms {
    fn get(&self, term_name: &str) -> Option<&FatTerm> {
        self.map.get(term_name)
    }

    fn edit(&mut self, term_name: &str, updated: &FatTerm) -> Result<(), KnowledgeBaseError> {
        match self
            .map
            .entry(term_name.to_string())
            .and_modify(|e| *e = updated.clone())
        {
            std::collections::hash_map::Entry::Occupied(_) => Ok(()),
            std::collections::hash_map::Entry::Vacant(_) => Err(KnowledgeBaseError::NotFound),
        }
    }

    fn put(&mut self, term_name: &str, term: FatTerm) -> Result<(), KnowledgeBaseError> {
        match self.map.entry(term_name.to_string()) {
            std::collections::hash_map::Entry::Occupied(_) => {
                Err(KnowledgeBaseError::AlreadyPresent)
            }
            std::collections::hash_map::Entry::Vacant(v) => {
                self.vec.push(term.meta.term.name.clone());
                v.insert(term);
                Ok(())
            }
        }
    }

    fn delete(&mut self, term_name: &str) {
        self.map.remove(term_name);
        let pos = self.vec.iter().position(|t| t == term_name).unwrap();
        self.vec.swap_remove(pos);
    }

    fn keys(&self) -> &Vec<String> {
        return &self.vec;
    }
}
