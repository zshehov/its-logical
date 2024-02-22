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

pub mod in_memory;
pub mod persistent;

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

pub enum ConsultResult {
    Success,
    Failure,
    Solutions
}

pub trait Consult {
    fn consult(&mut self, term: &BoundTerm) -> Vec<HashMap<String, String>>;
}

pub trait TermsStore: Get + Put + Keys + Delete + Consult {}

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
