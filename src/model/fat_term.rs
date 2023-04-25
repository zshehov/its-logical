use nom::{error::VerboseError, IResult};

use super::{
    comment::comment::{parse_comment, Comment},
    term::term::{parse_term, Term},
};

pub struct FatTerm {
    pub(crate) meta: Comment,
    pub(crate) term: Term,
}

impl FatTerm {
    pub(crate) fn new(meta: Comment, term: Term) -> Self {
        Self { meta, term }
    }
}

pub(crate) fn parse_fat_term<'a>(i: &'a str) -> IResult<&'a str, FatTerm, VerboseError<&str>> {
    let (leftover, meta) = parse_comment(i)?;
    let (leftover, term) = parse_term(leftover)?;

    Ok((leftover, FatTerm::new(meta, term)))
}
