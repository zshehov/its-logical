use nom::error::VerboseError;
use std::io;
use thiserror::Error;

use nom::bytes::complete::tag;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("parse failure: {}", verbose_err)]
    Parse { verbose_err: String },
}
