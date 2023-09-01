use nom::{
    bytes::complete::tag,
    error::VerboseError,
    multi::separated_list1,
    sequence::{separated_pair, terminated},
    IResult,
};

use super::{
    args_binding::ArgsBinding,
    bound_term::{parse_bound_term, BoundTerm},
};

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct Rule {
    pub head: ArgsBinding,
    pub body: Vec<BoundTerm>,
}

// parses "some_rule_name(SomeVar,someConst,_):=some_fact(SomeVar),some_rule(someConst,SomeVar)."
pub fn parse_rule(i: &str) -> IResult<&str, Rule, VerboseError<&str>> {
    let raw_rule = separated_pair(
        parse_bound_term,
        tag(":-"),
        terminated(separated_list1(tag(","), parse_bound_term), tag(".")),
    )(i);

    raw_rule.map(|(leftover, (head, body))| {
        (
            leftover,
            Rule {
                head: head.arg_bindings,
                body,
            },
        )
    })
}
