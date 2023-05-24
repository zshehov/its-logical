use nom::{
    bytes::complete::take_until,
    error::VerboseError,
    sequence::{delimited, tuple},
    IResult,
};

use super::args_binding::{parse_args_binding, ArgsBinding};

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub(crate) struct BoundTerm {
    pub(crate) name: String,
    pub(crate) arg_bindings: ArgsBinding,
}
impl BoundTerm {
    pub(crate) fn encode(&self) -> String {
        let mut encoded = String::new();
        encoded.push_str(&self.name);
        encoded.push_str("(");
        encoded.push_str(&self.arg_bindings.encode());
        encoded.push_str(")");
        encoded
    }
}
// parses "some_term_name(some_const,SomeVar,_)"
pub(crate) fn parse_bound_term<'a>(i: &'a str) -> IResult<&'a str, BoundTerm, VerboseError<&str>> {
    let name_and_args = tuple((
        take_until("("),
        delimited(
            nom::character::complete::char('('),
            parse_args_binding,
            nom::character::complete::char(')'),
        ),
    ))(i);

    name_and_args.map(|(leftover, (name, args))| {
        return (
            leftover,
            BoundTerm {
                name: name.to_string(),
                arg_bindings: args,
            },
        );
    })
}
