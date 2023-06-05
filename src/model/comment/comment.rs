use nom::{
    bytes::complete::{tag, take_till1, take_until},
    error::VerboseError,
    multi::{many0, separated_list0},
    sequence::{preceded, terminated, tuple},
    IResult,
};

use super::name_description::{parse_name_description, NameDescription};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Comment {
    pub(crate) term: NameDescription,
    pub(crate) args: Vec<NameDescription>,
    pub(crate) referred_by: Vec<String>,
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

        encoded.push_str("% @see ");
        encoded.push_str(&self.referred_by.join(",").to_string());
        encoded.push_str(NEWLINE);
        encoded
    }

    pub(crate) fn new(
        term: NameDescription,
        args: Vec<NameDescription>,
        referred_by: Vec<String>,
    ) -> Self {
        Self {
            term,
            args,
            referred_by,
        }
    }
}

pub(crate) fn parse_comment<'a>(i: &'a str) -> IResult<&'a str, Comment, VerboseError<&str>> {
    take_until("%!")(i)
        .and_then(|(leftover, _)| {
            tuple((
                term_definition_parser,
                args_definition_parser,
                referred_by_terms_parser,
            ))(leftover)
        })
        .map(|(leftover, (term, args, referred_by))| {
            (
                leftover,
                Comment {
                    term,
                    args,
                    referred_by,
                },
            )
        })
}

fn term_definition_parser<'a>(i: &'a str) -> IResult<&'a str, NameDescription, VerboseError<&str>> {
    preceded(tag("%! "), parse_name_description)(i)
}

fn args_definition_parser<'a>(
    i: &'a str,
) -> IResult<&'a str, Vec<NameDescription>, VerboseError<&str>> {
    many0(preceded(tag("% @arg "), parse_name_description))(i)
}

fn referred_by_terms_parser<'a>(i: &'a str) -> IResult<&'a str, Vec<String>, VerboseError<&str>> {
    preceded(
        tag("% @see "),
        terminated(
            separated_list0(tag(","), parse_to_owned_string),
            nom::character::complete::char('\n'),
        ),
    )(i)
}

fn parse_to_owned_string<'a>(i: &'a str) -> IResult<&'a str, String, VerboseError<&str>> {
    take_till1(|c| c == ',' || c == '\n')(i)
        .map(|(leftover, parsed)| (leftover, parsed.to_string()))
}

#[test]
fn test_referred_by_terms_parser() {
    let res = referred_by_terms_parser(
        r"% @see parent,male
",
    );
    assert_eq!(
        res,
        Ok(("", vec!["parent".to_string(), "male".to_string()]))
    );
    let res = referred_by_terms_parser(
        r"% @see parent
",
    );
    assert_eq!(res, Ok(("", vec!["parent".to_string()])));
    let res = referred_by_terms_parser(
        r"% @see 
",
    );
    assert_eq!(res, Ok(("", vec![])));
}
