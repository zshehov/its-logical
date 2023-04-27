use nom::{
    bytes::complete::tag, character::complete::newline, error::VerboseError, multi::many0,
    sequence::terminated, IResult,
};

use super::{
    args_binding::ArgsBinding,
    bound_term::{parse_bound_term, BoundTerm},
    rule::{parse_rule, Rule},
};

#[derive(PartialEq, Debug, Clone)]
pub(crate) struct Term {
    pub(crate) facts: Vec<ArgsBinding>,
    pub(crate) rules: Vec<Rule>,
}

impl Term {
    pub(crate) fn new(facts: Vec<ArgsBinding>, rules: Vec<Rule>) -> Self {
        Self { facts, rules }
    }
}

pub(crate) fn parse_term<'a>(i: &'a str) -> IResult<&'a str, Term, VerboseError<&str>> {
    let (leftover, facts) = many0(terminated(parse_fact, newline))(i)?;
    let (leftover, rules) = many0(terminated(parse_rule, newline))(leftover)?;

    Ok((
        leftover,
        Term {
            facts: facts.into_iter().map(|f| f.arg_bindings).collect(),
            rules,
        },
    ))
}

// parses "some_fact_name(SomeVar,someConst,_)."
fn parse_fact<'a>(i: &'a str) -> IResult<&'a str, BoundTerm, VerboseError<&str>> {
    terminated(parse_bound_term, tag("."))(i)
}

#[test]
fn test_parse_fact() {
    // Valid input
    assert_eq!(
        parse_fact("parent(john,mary)."),
        Ok((
            "",
            BoundTerm {
                name: "parent".to_string(),
                arg_bindings: ArgsBinding {
                    binding: vec![Some("john".to_string()), Some("mary".to_string())]
                },
            }
        ))
    );
}

#[test]
fn test_parse_term() {
    // Valid input
    assert_eq!(
        parse_term(
            r"parent(john,mary).
parent(bill,hilly).
parent(X,Y):-strong_match_in_dna(X,Y),older(X,Y)"
        ),
        Ok((
            "",
            Term {
                facts: vec![
                    ArgsBinding {
                        binding: vec![Some("john".to_string()), Some("mary".to_string())]
                    },
                    ArgsBinding {
                        binding: vec![Some("bill".to_string()), Some("hilly".to_string())]
                    }
                ],
                rules: vec![Rule {
                    arg_bindings: ArgsBinding {
                        binding: vec![Some("X".to_string()), Some("Y".to_string())],
                    },
                    body: vec![
                        BoundTerm {
                            name: "strong_match_in_dna".to_string(),
                            arg_bindings: ArgsBinding {
                                binding: vec![Some("X".to_string()), Some("Y".to_string())]
                            }
                        },
                        BoundTerm {
                            name: "older".to_string(),
                            arg_bindings: ArgsBinding {
                                binding: vec![Some("X".to_string()), Some("Y".to_string())]
                            }
                        }
                    ]
                }],
            }
        ))
    );
}
