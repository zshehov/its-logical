use std::collections::HashSet;

use nom::{error::VerboseError, IResult};

use super::{
    comment::{
        name_description::NameDescription,
        {parse_comment, Comment},
    },
    term::term::{parse_term, Term},
};

#[derive(Clone, Debug, PartialEq)]
pub struct FatTerm {
    pub meta: Comment,
    pub term: Term,
}

impl FatTerm {
    pub fn new(meta: Comment, term: Term) -> Self {
        Self { meta, term }
    }

    pub fn encode(&self) -> String {
        let mut encoded = String::new();
        encoded.push_str(&self.meta.encode());
        encoded.push_str(&self.term.encode(&self.meta.term.name));
        encoded
    }

    pub fn add_referred_by(&mut self, term_name: &String) -> bool {
        if !self.meta.referred_by.contains(term_name) {
            self.meta.referred_by.push(term_name.to_owned());
            return true;
        }
        false
    }

    pub fn remove_referred_by(&mut self, term_name: &str) -> bool {
        if let Some(idx) = self.meta.referred_by.iter().position(|x| x == term_name) {
            self.meta.referred_by.remove(idx);
            return true;
        }
        false
    }

    pub fn rename_referred_by(&mut self, from: &str, to: &str) -> bool {
        if let Some(idx) = self.meta.referred_by.iter().position(|x| x == from) {
            *self.meta.referred_by.get_mut(idx).unwrap() = to.to_owned();
            return true;
        }
        false
    }

    pub fn mentioned_terms(&self) -> HashSet<String> {
        let mut mentioned_terms = HashSet::<String>::new();

        for rule in self.term.rules.iter() {
            for body_term in &rule.body {
                mentioned_terms.insert(body_term.name.clone());
            }
        }
        mentioned_terms
    }
}

impl Default for FatTerm {
    fn default() -> Self {
        FatTerm::new(
            Comment::new(NameDescription::new("", ""), &[], &[]),
            Term::new(&[], &[]),
        )
    }
}

pub fn parse_fat_term(i: &str) -> IResult<&str, FatTerm, VerboseError<&str>> {
    let (leftover, meta) = parse_comment(i)?;
    let (leftover, term) = parse_term(leftover)?;

    Ok((leftover, FatTerm::new(meta, term)))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::model::term::{
        args_binding::ArgsBinding, bound_term::BoundTerm, rule::Rule,
    };
    #[test]
    fn test_parse_encode() {
        let input = r"% -father a father is a parent that's male
% @arg FatherName the name of the father
% @arg ChildName the name of the child
% @see parent,male
father(stefan,petko).
father(hristo,stoichko).
father(Father,Child):-parent(Father,Child),male(Father).
";
        let parsed = parse_fat_term(input);

        let expected = FatTerm::new(
            Comment::new(
                NameDescription::new("father", "a father is a parent that's male"),
                &[
                    NameDescription::new("FatherName", "the name of the father"),
                    NameDescription::new("ChildName", "the name of the child"),
                ],
                &["parent".to_string(), "male".to_string()],
            ),
            Term::new(
                &[
                    ArgsBinding {
                        binding: vec!["stefan".to_string(), "petko".to_string()],
                    },
                    ArgsBinding {
                        binding: vec!["hristo".to_string(), "stoichko".to_string()],
                    },
                ],
                &[Rule {
                    head: ArgsBinding {
                        binding: vec!["Father".to_string(), "Child".to_string()],
                    },
                    body: vec![
                        BoundTerm {
                            name: "parent".to_string(),
                            arg_bindings: ArgsBinding {
                                binding: vec!["Father".to_string(), "Child".to_string()],
                            },
                        },
                        BoundTerm {
                            name: "male".to_string(),
                            arg_bindings: ArgsBinding {
                                binding: vec!["Father".to_string()],
                            },
                        },
                    ],
                }],
            ),
        );
        let encoded = expected.encode();
        assert_eq!(encoded, input);
        assert_eq!(parsed, Ok(("", expected)));
    }
}
