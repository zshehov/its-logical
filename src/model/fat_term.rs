use nom::{error::VerboseError, IResult};

use super::{
    comment::{
        comment::{parse_comment, Comment},
        name_description::NameDescription,
    },
    term::{
        args_binding::ArgsBinding,
        term::{parse_term, Term},
    },
};

#[derive(Clone, Debug, PartialEq)]
pub struct FatTerm {
    pub(crate) meta: Comment,
    pub(crate) term: Term,
}

impl FatTerm {
    pub(crate) fn new(meta: Comment, term: Term) -> Self {
        Self { meta, term }
    }

    pub(crate) fn encode(&self) -> String {
        let mut encoded = String::new();
        encoded.push_str(&self.meta.encode());
        encoded.push_str(&self.term.encode(&self.meta.term.name));
        encoded
    }
}

impl Default for FatTerm {
    fn default() -> Self {
        FatTerm::new(
            Comment::new(NameDescription::new("", ""), vec![], vec![]),
            Term::new(vec![ArgsBinding { binding: vec![] }], vec![]),
        )
    }
}

pub(crate) fn parse_fat_term<'a>(i: &'a str) -> IResult<&'a str, FatTerm, VerboseError<&str>> {
    let (leftover, meta) = parse_comment(i)?;
    let (leftover, term) = parse_term(leftover)?;

    Ok((leftover, FatTerm::new(meta, term)))
}

#[test]
fn test_parse_encode() {
    use crate::model::term::bound_term::BoundTerm;
    use crate::model::term::rule::Rule;

    let input = r"%! father a father is a parent that's male
% @arg FatherName the name of the father
% @arg ChildName the name of the child
% @see parent,male
father(stefan,petko).
father(hristo,stoichko).
father(Father,Child):-parent(Father,Child),male(Father)
";
    let parsed = parse_fat_term(input);

    let expected = FatTerm::new(
        Comment::new(
            NameDescription::new("father", "a father is a parent that's male"),
            vec![
                NameDescription::new("FatherName", "the name of the father"),
                NameDescription::new("ChildName", "the name of the child"),
            ],
            vec!["parent".to_string(), "male".to_string()],
        ),
        Term::new(
            vec![
                ArgsBinding {
                    binding: vec!["stefan".to_string(), "petko".to_string()],
                },
                ArgsBinding {
                    binding: vec!["hristo".to_string(), "stoichko".to_string()],
                },
            ],
            vec![Rule {
                arg_bindings: ArgsBinding {
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
