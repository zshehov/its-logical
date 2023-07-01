use bincode::{config, decode_from_std_read, encode_into_std_write, Encode};
use bincode_derive::Decode;


use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use crate::model::fat_term::{parse_fat_term, FatTerm};

#[derive(Debug)]
pub enum KnowledgeBaseError {
    NotFound,
    AlreadyPresent,
    //TODO:  InvalidTerm,
}

pub trait TermsKnowledgeBase {
    fn get(&self, term_name: &str) -> Option<FatTerm>;
    // the term.meta.term.name takes precedence to the provided term_name
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

    fn put(&mut self, term_name: &str, term: FatTerm) -> Result<(), KnowledgeBaseError> {
        match self.map.entry(term_name.to_string()) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                *e.get_mut() = term;
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(term);
            }
        }

        Ok(())
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

#[derive(Decode, Encode, Clone)]
struct DescriptorEntry {
    name: String,
    offset: usize,

    len: usize,
    // this field should be skipped during encoding/decoding - couldn't find a way to do that with bincode
    is_deleted: bool,
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

        let mut descriptor_vec = if !descriptor_path.exists() {
            File::create(&descriptor_path).unwrap();
            vec![]
        } else {
            let descriptor = OpenOptions::new().read(true).open(descriptor_path).unwrap();

            let mut descriptor = BufReader::new(descriptor);
            let descriptor_vec: Vec<DescriptorEntry> =
                decode_from_std_read(&mut descriptor, config::standard()).unwrap();
            descriptor_vec
        };

        // TODO: use drain_filter when it's stable
        descriptor_vec = descriptor_vec
            .into_iter()
            .filter(|x| !x.is_deleted)
            .collect();

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

    fn edit(
        &mut self,
        term_name: &str,
        term_idx: usize,
        updated: &FatTerm,
    ) -> Result<(), KnowledgeBaseError> {
        let entry = &mut self.descriptor[term_idx];
        let original_len = entry.len;
        let updated_encoded = &updated.encode();

        let len_diff: i64 = updated_encoded.len() as i64 - original_len as i64;

        self.buffer
            .replace_range(entry.offset..entry.offset + entry.len, updated_encoded);

        entry.len = updated_encoded.len();

        self.descriptor[term_idx].name = updated.meta.term.name.to_owned();
        for desriptor_entry in self.descriptor[term_idx + 1..].iter_mut() {
            let mut adjusted_offset = desriptor_entry.offset as i64;
            adjusted_offset += len_diff;

            desriptor_entry.offset = adjusted_offset as usize;
        }
        self.index.remove(term_name);
        self.index
            .insert(updated.meta.term.name.to_string(), term_idx);
        self.keys[term_idx] = updated.meta.term.name.to_string();

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
            is_deleted: false,
        });

        self.keys.push(term_name.to_string());

        self.buffer.push_str(&encoded_term);
        Ok(())
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

    fn put(&mut self, term_name: &str, term: FatTerm) -> Result<(), KnowledgeBaseError> {
        match self.index.get(term_name) {
            Some(&term_idx) => self.edit(term_name, term_idx, &term),
            None => self.put(&term.meta.term.name.clone(), term),
        }
    }

    fn keys(&self) -> &Vec<String> {
        return &self.keys;
    }

    // delete doesn't delete the descriptor entry for the record - rather it just sets its len to 0
    fn delete(&mut self, term_name: &str) {
        let deleted_entry_idx = self.index.get(term_name).unwrap().to_owned();
        let deleted_entry = self.descriptor[deleted_entry_idx].to_owned();

        if let Some(deleted_entry) = self.descriptor.get_mut(deleted_entry_idx) {
            *deleted_entry = DescriptorEntry {
                name: "".to_string(),
                offset: 0,
                len: 0,
                is_deleted: true,
            }
        }

        self.buffer.replace_range(
            deleted_entry.offset..deleted_entry.offset + deleted_entry.len,
            "",
        );

        for desriptor_entry in self.descriptor[deleted_entry_idx + 1..].iter_mut() {
            let mut adjusted_offset = desriptor_entry.offset as i64;
            adjusted_offset -= deleted_entry.len as i64;

            desriptor_entry.offset = adjusted_offset as usize;
        }
        self.index.remove(term_name);
        self.keys.remove(deleted_entry_idx);
    }
}
