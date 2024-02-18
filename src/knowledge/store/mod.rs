use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{self, BufReader, BufWriter},
    path::{Path, PathBuf},
};

use bincode::{config, decode_from_std_read, Encode, encode_into_std_write};
use bincode_derive::Decode;
use scryer_prolog::machine::Machine;
use scryer_prolog::machine::parsed_results::{QueryResolution, Value};

use crate::knowledge::model::fat_term::{FatTerm, parse_fat_term};
use crate::knowledge::model::term::bound_term::BoundTerm;

#[derive(Debug)]
pub enum Error {
    NotFound,
    AlreadyPresent,
    //TODO:  InvalidTerm,
}

pub trait Get {
    fn get(&self, term_name: &str) -> Option<FatTerm>;
}

pub trait Put {
    // the term.meta.term.name takes precedence to the provided term_name
    fn put(&mut self, term_name: &str, term: FatTerm) -> Result<(), Error>;
}

pub trait Keys {
    fn keys(&self) -> &Vec<String>;
}

pub trait Delete {
    fn delete(&mut self, term_name: &str);
}

pub trait Load {
    type Store: Get + Put + Keys + Delete;

    fn load(path: &Path) -> Self::Store;
}

pub trait Consult {
    fn consult(&mut self, term: &BoundTerm) -> Vec<HashMap<String, String>>;
}

pub trait TermsStore: Get + Put + Keys + Delete + Consult {}

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
    fn put(&mut self, term_name: &str, term: FatTerm) -> Result<(), Error> {
        if self.map.contains_key(term_name) {
            self.map.remove(term_name);
        }
        self.map.insert(term.meta.term.name.clone(), term);

        Ok(())
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
    fn consult(&mut self, term: &BoundTerm) -> Vec<HashMap<String, String>> {
        todo!()
    }
}

impl TermsStore for InMemoryTerms {}

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

pub struct PersistentTermsWithEngine {
    terms: PersistentMemoryTerms,
    engine: Machine,
}

impl Get for PersistentTermsWithEngine {
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        self.terms.get(term_name)
    }
}

impl Put for PersistentTermsWithEngine {
    fn put(&mut self, term_name: &str, term: FatTerm) -> Result<(), Error> {
        self.terms.put(term_name, term).and_then(|_| Ok({
            // TODO: check if it's too slow to load all of the buffer every time a term is put
            // TODO: maybe expose `.flush` that guarantees that the buffer has been loaded in the engine
            self.engine.consult_module_string("knowledge", self.terms.buffer.clone())
        }))
    }
}

impl Keys for PersistentTermsWithEngine {
    fn keys(&self) -> &Vec<String> {
        self.terms.keys()
    }
}

impl Delete for PersistentTermsWithEngine {
    fn delete(&mut self, term_name: &str) {
        self.terms.delete(term_name);
        // TODO: check if it's too slow to load all of the buffer every time a term is deleted
        // TODO: maybe expose `.flush` that guarantees that the buffer has been loaded in the engine
        self.engine.consult_module_string("knowledge", self.terms.buffer.clone())
    }
}

impl Consult for PersistentTermsWithEngine {
    fn consult(&mut self, term: &BoundTerm) -> Vec<HashMap<String, String>> {
        // TODO: choose where to put the persist, maybe rather in put and delete
        self.terms.persist();
        // the term must be finished with a '.' to be a valid prolog query
        let result = self.engine.run_query(term.encode() + ".");
        return match result {
            Ok(resolution) => {
                match resolution {
                    QueryResolution::True => {vec![]}
                    QueryResolution::False => {vec![]}
                    QueryResolution::Matches(matches) => {
                        // TODO: decide if it's worth it to handle something other than String
                        let consult_result = matches.iter().map(|x| {
                            let mut bound = HashMap::with_capacity(x.bindings.len());
                            for binding in &x.bindings {
                                if let Value::String(s) = binding.1 {
                                    bound.insert(binding.0.to_owned(), s.to_owned());
                                } else {
                                    panic!("all values are currently expected to be strings");
                                }
                            }
                            return bound;
                        }).collect();

                        consult_result
                    }
                }
            }
            Err(e) => {
                panic!("{:?}", e)
            }
        };
    }
}

impl TermsStore for PersistentTermsWithEngine {}

impl Load for PersistentTermsWithEngine {
    type Store = PersistentTermsWithEngine;

    fn load(path: &Path) -> Self::Store {
        let terms = PersistentMemoryTerms::load(path);
        let mut engine = Machine::new_lib();
        engine.consult_module_string("knowledge", terms.buffer.clone());

        PersistentTermsWithEngine {
            terms,
            engine,
        }
    }
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

    pub fn new(path: &Path) -> Self {
        let descriptor_path = path.join(DESCRIPTOR_NAME);

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

        descriptor_vec.retain(|x| !x.is_deleted);

        let mut index = HashMap::new();
        for (entry_idx, entry) in descriptor_vec.iter().enumerate() {
            index.insert(entry.name.clone(), entry_idx);
        }
        let mut keys = Vec::with_capacity(descriptor_vec.len());

        for entry in &descriptor_vec {
            keys.push(entry.name.clone());
        }

        let page_path = path.join(PAGE_NAME);
        let page_content = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(page_path)
            .unwrap();
        let page_content = io::read_to_string(page_content).unwrap();

        Self {
            index,
            descriptor: descriptor_vec,
            base_path: path.to_owned(),
            buffer: page_content,
            keys,
        }
    }

    fn edit(&mut self, term_name: &str, term_idx: usize, updated: &FatTerm) -> Result<(), Error> {
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
        let keys_idx = self.keys.iter().position(|name| name == term_name).unwrap();
        self.keys[keys_idx] = updated.meta.term.name.to_string();

        Ok(())
    }

    fn create(&mut self, term_name: &str, term: FatTerm) -> Result<(), Error> {
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

impl Get for PersistentMemoryTerms {
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
}

impl Put for PersistentMemoryTerms {
    fn put(&mut self, term_name: &str, term: FatTerm) -> Result<(), Error> {
        match self.index.get(term_name) {
            Some(&term_idx) => self.edit(term_name, term_idx, &term),
            None => self.create(&term.meta.term.name.clone(), term),
        }
    }
}

impl Keys for PersistentMemoryTerms {
    fn keys(&self) -> &Vec<String> {
        &self.keys
    }
}

impl Delete for PersistentMemoryTerms {
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

        let keys_idx = self.keys.iter().position(|name| name == term_name).unwrap();
        self.keys.remove(keys_idx);
    }
}

impl Load for PersistentMemoryTerms {
    type Store = PersistentMemoryTerms;

    fn load(path: &Path) -> Self::Store {
        PersistentMemoryTerms::new(path)
    }
}