use nom::{
    bytes::complete::{tag, take_until},
    error::VerboseError,
    multi::many0,
    sequence::{preceded, tuple},
    IResult,
};

use super::name_description::{parse_name_description, NameDescription};

#[derive(Clone)]
pub(crate) struct Comment {
    pub(crate) term: NameDescription,
    pub(crate) args: Vec<NameDescription>,
}

impl Comment {
    pub(crate) fn new(term: NameDescription, args: Vec<NameDescription>) -> Self {
        Self { term, args }
    }
}

pub(crate) fn parse_comment<'a>(i: &'a str) -> IResult<&'a str, Comment, VerboseError<&str>> {
    take_until("%!")(i)
        .and_then(|(leftover, _)| tuple((term_definition_parser, args_definition_parser))(leftover))
        .map(|(leftover, (term, args))| (leftover, Comment { term, args }))
}

fn term_definition_parser<'a>(i: &'a str) -> IResult<&'a str, NameDescription, VerboseError<&str>> {
    preceded(tag("%! "), parse_name_description)(i)
}

fn args_definition_parser<'a>(
    i: &'a str,
) -> IResult<&'a str, Vec<NameDescription>, VerboseError<&str>> {
    many0(preceded(tag("% @arg "), parse_name_description))(i)
}
