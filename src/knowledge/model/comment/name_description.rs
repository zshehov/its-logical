use std::fmt;

use nom::bytes::complete::{tag, take_until};
use nom::error::VerboseError;
use nom::sequence::{separated_pair, terminated};
use nom::IResult;

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct NameDescription {
    pub name: String,
    pub desc: String,
}

impl NameDescription {
    pub fn new(name: &str, desc: &str) -> Self {
        Self {
            name: name.to_string(),
            desc: desc.to_string(),
        }
    }

    pub fn encode(&self) -> String {
        let mut encoded = String::with_capacity(self.name.len() + 1 + self.desc.len());

        encoded.push_str(&self.name);
        encoded.push(' ');
        encoded.push_str(&self.desc);
        encoded
    }
}

impl fmt::Display for NameDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}| {}", self.name, self.desc)
    }
}

pub fn parse_name_description<'a>(
    i: &'a str,
) -> IResult<&'a str, NameDescription, VerboseError<&str>> {
    separated_pair(
        take_until(" "),
        tag(" "),
        terminated(take_until("\n"), nom::character::complete::char('\n')),
    )(i)
    .map(|(leftover, (name, desc))| {
        (
            leftover,
            NameDescription {
                name: name.to_owned(),
                desc: desc.to_owned(),
            },
        )
    })
}
