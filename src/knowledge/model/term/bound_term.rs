use nom::{
    bytes::complete::take_until,
    error::VerboseError,
    sequence::{delimited, tuple},
    IResult,
};

use super::args_binding::{parse_args_binding, ArgsBinding};

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct BoundTerm {
    pub name: String,
    pub arg_bindings: ArgsBinding,
}

impl BoundTerm {
    pub fn new(name: &str, arg_bindings: ArgsBinding) -> Self {
        Self {
            name: name.to_string(),
            arg_bindings,
        }
    }
    pub fn encode(&self) -> String {
        let mut encoded = String::new();
        encoded.push_str(&self.name);
        encoded.push('(');
        encoded.push_str(&self.arg_bindings.encode());
        encoded.push(')');
        encoded
    }
}

// parses "some_term_name(some_const,SomeVar,_)"
pub fn parse_bound_term(i: &str) -> IResult<&str, BoundTerm, VerboseError<&str>> {
    let name_and_args = tuple((
        take_until("("),
        delimited(
            nom::character::complete::char('('),
            parse_args_binding,
            nom::character::complete::char(')'),
        ),
    ))(i);

    name_and_args.map(|(leftover, (name, args))| {
        (
            leftover,
            BoundTerm {
                name: name.to_string(),
                arg_bindings: args,
            },
        )
    })
}
