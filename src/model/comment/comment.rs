use nom::{
    bytes::complete::{tag, take_until},
    error::VerboseError,
    multi::many0,
    sequence::{preceded, tuple},
    IResult,
};

use super::name_description::{parse_name_description, NameDescription};

#[derive(Clone,Debug,PartialEq)]
pub(crate) struct Comment {
    pub(crate) term: NameDescription,
    pub(crate) args: Vec<NameDescription>,
}

const NEWLINE: &str = r"
";

impl Comment {
    pub(crate) fn encode(&self) -> String {
        let term_encoded = self.term.encode();
        let mut encoded = String::with_capacity(term_encoded.len() + "%! ".len() + 1);
        encoded.push_str("%! ");
        encoded.push_str(&term_encoded);
        encoded.push_str(NEWLINE);

        for arg in &self.args {
            encoded.push_str("% @arg ");
            encoded.push_str(&arg.encode());
            encoded.push_str(NEWLINE);
        }
        encoded
    }

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
