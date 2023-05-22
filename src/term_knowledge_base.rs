use bincode::{config, decode_from_std_read, encode_into_std_write, Encode};
use bincode_derive::Decode;

use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use crate::model::{
    fat_term::{parse_fat_term, FatTerm},
    term,
};

pub enum KnowledgeBaseError {
    NotFound,
    AlreadyPresent,
    //TODO:  InvalidTerm,
}

pub trait TermsKnowledgeBase {
    fn get(&self, term_name: &str) -> Option<FatTerm>;
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
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        self.map.get(term_name).cloned()
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

const PAGE_NAME: &str = "page.pl";
const DESCRIPTOR_NAME: &str = "descriptor";

#[derive(Decode, Encode)]
struct DescriptorEntry {
    name: String,
    offset: usize,
    len: usize,
}

pub struct PersistentMemoryTerms {
    // TODO: change this index to a DB - lmdb is probably best
    index: HashMap<String, usize>,
    descriptor: Vec<DescriptorEntry>,
    // TODO: this is temorary, as you can get the same from the descriptor
    keys: Vec<String>,
    base_path: PathBuf,
    buffer: String,
}
impl Drop for PersistentMemoryTerms {
    fn drop(&mut self) {
        self.persist()
    }
}

impl PersistentMemoryTerms {
    fn persist(&self) {
        let descriptor = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(self.base_path.join(DESCRIPTOR_NAME))
            .unwrap();
        let mut buf_writer = BufWriter::new(descriptor);

        encode_into_std_write(&self.descriptor, &mut buf_writer, config::standard()).unwrap();

        fs::write(self.base_path.join(PAGE_NAME), &self.buffer).unwrap();
    }

    pub fn new(base_path: &PathBuf) -> Self {
        let descriptor_path = base_path.join(DESCRIPTOR_NAME);

        let descriptor_vec = if !descriptor_path.exists() {
            File::create(&descriptor_path).unwrap();
            vec![]
        } else {
            let descriptor = OpenOptions::new().read(true).open(descriptor_path).unwrap();

            let mut descriptor = BufReader::new(descriptor);
            let descriptor_vec: Vec<DescriptorEntry> =
                decode_from_std_read(&mut descriptor, config::standard()).unwrap();
            descriptor_vec
        };

        let mut index = HashMap::new();
        for (entry_idx, entry) in descriptor_vec.iter().enumerate() {
            index.insert(entry.name.clone(), entry_idx);
        }
        let mut keys = Vec::with_capacity(descriptor_vec.len());

        for entry in &descriptor_vec {
            keys.push(entry.name.clone());
        }

        let page_content = fs::read_to_string(base_path.join(PAGE_NAME)).unwrap();

        Self {
            index,
            descriptor: descriptor_vec,
            base_path: base_path.to_owned(),
            buffer: page_content,
            keys,
        }
    }
}

impl TermsKnowledgeBase for PersistentMemoryTerms {
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        match self.index.get(term_name) {
            Some(offset) => {
                let entry = &self.descriptor[*offset];
                let raw_term = &self.buffer[entry.offset..entry.offset + entry.len];

                let (_, fat_term) = parse_fat_term(raw_term).unwrap();
                Some(fat_term)
            }
            None => None,
        }
    }

    fn edit(&mut self, term_name: &str, updated: &FatTerm) -> Result<(), KnowledgeBaseError> {
        let offset = self.index.get(term_name).unwrap().to_owned();
        let entry = &mut self.descriptor[offset];
        let original_len = entry.len;
        let updated_encoded = &updated.encode();

        let len_diff: i64 = updated_encoded.len() as i64 - original_len as i64;

        self.buffer
            .replace_range(entry.offset..entry.offset + entry.len, updated_encoded);

        entry.len = updated_encoded.len();

        self.descriptor[offset].name = updated.meta.term.name.to_owned();
        for desriptor_entry in self.descriptor[offset + 1..].iter_mut() {
            let mut adjusted_offset = desriptor_entry.offset as i64;
            adjusted_offset += len_diff;

            desriptor_entry.offset = adjusted_offset as usize;
        }
        self.index.remove(term_name);
        self.index
            .insert(updated.meta.term.name.to_string(), offset);
        self.keys[offset] = updated.meta.term.name.to_string();

        Ok(())
    }

    fn put(&mut self, term_name: &str, term: FatTerm) -> Result<(), KnowledgeBaseError> {
        let encoded_term = term.encode();

        let mut new_entry_offset = 0;
        let new_entry_len = encoded_term.len();
        if let Some(entry) = self.descriptor.last() {
            new_entry_offset = entry.offset + entry.len;
        }

        self.index
            .insert(term_name.to_string(), self.descriptor.len());
        self.descriptor.push(DescriptorEntry {
            name: term_name.to_string(),
            offset: new_entry_offset,
            len: new_entry_len,
        });

        self.keys.push(term_name.to_string());

        self.buffer.push_str(&encoded_term);
        Ok(())
    }

    fn keys(&self) -> &Vec<String> {
        return &self.keys;
    }

    fn delete(&mut self, term_name: &str) {
        todo!()
    }
}
