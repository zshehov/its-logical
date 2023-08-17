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
pub struct Term {
    pub facts: Vec<ArgsBinding>,
    pub rules: Vec<Rule>,
}

const NEWLINE: &str = r"
";
const END_OF_CLAUSE: &str = r".";
const END_OF_FACT: &str = r").
";
const END_OF_RULE_HEAD: &str = r"):-";

impl Term {
    pub fn new(facts: &[ArgsBinding], rules: &[Rule]) -> Self {
        Self {
            facts: facts.to_vec(),
            rules: rules.to_vec(),
        }
    }

    pub fn encode(&self, term_name: &str) -> String {
        let mut encoded = String::new();
        let term_name_prefix = term_name.to_owned() + "(";

        for arg_binding in &self.facts {
            encoded.push_str(&term_name_prefix);
            encoded.push_str(&arg_binding.encode());
            encoded.push_str(END_OF_FACT);
        }

        for rule in &self.rules {
            encoded.push_str(&term_name_prefix);

            encoded.push_str(&rule.head.encode());
            encoded.push_str(END_OF_RULE_HEAD);

            let body_entries: Vec<String> = rule.body.iter().map(|b| b.encode()).collect();
            encoded.push_str(&body_entries.join(","));
            encoded.push_str(END_OF_CLAUSE);
            encoded.push_str(NEWLINE);
        }
        encoded
    }
}

pub fn parse_term<'a>(i: &'a str) -> IResult<&'a str, Term, VerboseError<&str>> {
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
                    binding: vec!["john".to_string(), "mary".to_string()]
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
parent(X,Y):-strong_match_in_dna(X,Y),older(X,Y).
"
        ),
        Ok((
            "",
            Term {
                facts: vec![
                    ArgsBinding {
                        binding: vec!["john".to_string(), "mary".to_string()]
                    },
                    ArgsBinding {
                        binding: vec!["bill".to_string(), "hilly".to_string()]
                    }
                ],
                rules: vec![Rule {
                    head: ArgsBinding {
                        binding: vec!["X".to_string(), "Y".to_string()],
                    },
                    body: vec![
                        BoundTerm {
                            name: "strong_match_in_dna".to_string(),
                            arg_bindings: ArgsBinding {
                                binding: vec!["X".to_string(), "Y".to_string()]
                            }
                        },
                        BoundTerm {
                            name: "older".to_string(),
                            arg_bindings: ArgsBinding {
                                binding: vec!["X".to_string(), "Y".to_string()]
                            }
                        }
                    ]
                }],
            }
        ))
    );
}

#[test]
fn test_encode_term() {
    let term = Term {
        facts: vec![
            ArgsBinding {
                binding: vec!["john".to_string(), "mary".to_string()],
            },
            ArgsBinding {
                binding: vec!["bill".to_string(), "hilly".to_string()],
            },
        ],
        rules: vec![Rule {
            head: ArgsBinding {
                binding: vec!["X".to_string(), "Y".to_string()],
            },
            body: vec![
                BoundTerm {
                    name: "strong_match_in_dna".to_string(),
                    arg_bindings: ArgsBinding {
                        binding: vec!["X".to_string(), "Y".to_string()],
                    },
                },
                BoundTerm {
                    name: "older".to_string(),
                    arg_bindings: ArgsBinding {
                        binding: vec!["X".to_string(), "Y".to_string()],
                    },
                },
            ],
        }],
    };

    assert_eq!(
        term.encode("parent"),
        r"parent(john,mary).
parent(bill,hilly).
parent(X,Y):-strong_match_in_dna(X,Y),older(X,Y).
"
    );
}
