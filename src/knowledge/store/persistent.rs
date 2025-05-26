use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::knowledge::model::fat_term::{parse_fat_term, FatTerm};
use crate::knowledge::model::term::bound_term::BoundTerm;
use crate::knowledge::store::{
    Consult, Delete, DescriptorEntry, Error, Get, Keys, Load, Put, TermsStore, DESCRIPTOR_NAME,
    PAGE_NAME,
};
use bincode::{config, decode_from_std_read, encode_into_std_write};
use scryer_prolog::Machine;
use scryer_prolog::Term::Atom;
use scryer_prolog::{LeafAnswer, MachineBuilder};

pub struct TermsWithEngine {
    terms: Terms,
    engine: Machine,
}

impl Get for TermsWithEngine {
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        self.terms.get(term_name)
    }
}

impl Put for TermsWithEngine {
    fn put(&mut self, term_name: &str, term: FatTerm) -> Result<(), Error> {
        self.terms.put(term_name, term).and_then(|_| {
            Ok({
                // TODO: check if it's too slow to load all of the buffer every time a term is put
                // TODO: maybe expose `.flush` that guarantees that the buffer has been loaded in the engine
                self.engine
                    .load_module_string("knowledge", self.terms.buffer.clone())
            })
        })
    }
}

impl Keys for TermsWithEngine {
    fn keys(&self) -> &Vec<String> {
        self.terms.keys()
    }
}

impl Delete for TermsWithEngine {
    fn delete(&mut self, term_name: &str) {
        self.terms.delete(term_name);
        // TODO: check if it's too slow to load all of the buffer every time a term is deleted
        // TODO: maybe expose `.flush` that guarantees that the buffer has been loaded in the engine
        self.engine
            .load_module_string("knowledge", self.terms.buffer.clone())
    }
}

impl Consult for TermsWithEngine {
    fn consult(&mut self, term: &BoundTerm) -> Vec<HashMap<String, String>> {
        // the term must be finished with a '.' to be a valid prolog query
        let mut consult_results = vec![];
        let results = self.engine.run_query(term.encode() + ".");
        for binding in results {
            match binding {
                Ok(b) => match b {
                    LeafAnswer::True => {
                        // TODO: represent success
                    }
                    LeafAnswer::False => {
                        // TODO: represent failure
                    }
                    LeafAnswer::Exception(_) => {}
                    LeafAnswer::LeafAnswer {
                        bindings: arg_binding,
                        ..
                    } => {
                        let mut bound = HashMap::with_capacity(arg_binding.len());
                        for binding in &arg_binding {
                            if let Atom(s) = binding.1 {
                                bound.insert(binding.0.to_owned(), s.to_owned());
                            } else {
                                panic!("all values are currently expected to be strings");
                            }
                        }
                        consult_results.push(bound);
                    }
                },
                Err(e) => {
                    panic!("{:?}", e)
                }
            };
        }
        consult_results
    }
}

impl TermsStore for TermsWithEngine {}

impl Load for TermsWithEngine {
    type Store = TermsWithEngine;

    fn load(path: &Path) -> Self::Store {
        let terms = Terms::load(path);
        let builder = MachineBuilder::default();
        let mut engine = builder.build();
        engine.load_module_string("knowledge", terms.buffer.clone());

        TermsWithEngine { terms, engine }
    }
}

pub struct Terms {
    // TODO: change this index to a DB - lmdb is probably best
    index: HashMap<String, usize>,
    descriptor: Vec<DescriptorEntry>,
    // TODO: this is temorary, as you can get the same from the descriptor
    keys: Vec<String>,
    base_path: PathBuf,
    buffer: String,
}

impl Drop for Terms {
    fn drop(&mut self) {
        self.persist()
    }
}

impl Terms {
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

impl Get for Terms {
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

impl Put for Terms {
    fn put(&mut self, term_name: &str, term: FatTerm) -> Result<(), Error> {
        match self.index.get(term_name) {
            Some(&term_idx) => self.edit(term_name, term_idx, &term),
            None => self.create(&term.meta.term.name.clone(), term),
        }
    }
}

impl Keys for Terms {
    fn keys(&self) -> &Vec<String> {
        &self.keys
    }
}

impl Delete for Terms {
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

        for descriptor_entry in self.descriptor[deleted_entry_idx + 1..].iter_mut() {
            descriptor_entry.offset -= deleted_entry.len;
        }
        self.index.remove(term_name);

        let keys_idx = self.keys.iter().position(|name| name == term_name).unwrap();
        self.keys.remove(keys_idx);
    }
}

impl Load for Terms {
    type Store = Terms;

    fn load(path: &Path) -> Self::Store {
        Terms::new(path)
    }
}
