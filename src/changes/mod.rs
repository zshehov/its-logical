pub mod change;
pub mod deletion;
mod terms_cache;

#[cfg(test)]
mod tests {
    use crate::changes::change::{Apply, ArgsChange, Change};
    use crate::knowledge::model::{
        comment::{name_description::NameDescription, Comment},
        fat_term::FatTerm,
        term::{args_binding::ArgsBinding, bound_term::BoundTerm, rule::Rule, Term},
    };
    use std::collections::HashSet;

    fn create_related_test_term() -> FatTerm {
        FatTerm::new(
            Comment::new(
                NameDescription::new("first_related", "related description"),
                &[NameDescription::new("FirstArg", "First arg's description")],
                &[],
            ),
            Term::new(
                &[ArgsBinding {
                    binding: vec!["fact_value".to_string()],
                }],
                &[
                    Rule {
                        head: ArgsBinding {
                            binding: vec!["FirstHeadArg".to_string()],
                        },
                        body: vec![
                            BoundTerm {
                                name: "first_body_term".to_string(),
                                arg_bindings: ArgsBinding {
                                    binding: vec!["with_some_arg".to_string()],
                                },
                            },
                            BoundTerm {
                                name: "second_body_term".to_string(),
                                arg_bindings: ArgsBinding {
                                    binding: vec!["with_some_arg2".to_string()],
                                },
                            },
                        ],
                    },
                    Rule {
                        head: ArgsBinding {
                            binding: vec!["FirstHeadArg2".to_string()],
                        },
                        body: vec![
                            BoundTerm {
                                name: "test".to_string(),
                                arg_bindings: ArgsBinding {
                                    binding: vec!["FirstHeadArg2".to_string()],
                                },
                            },
                            BoundTerm {
                                name: "second_body_term2".to_string(),
                                arg_bindings: ArgsBinding {
                                    binding: vec!["with_some_arg2".to_string()],
                                },
                            },
                        ],
                    },
                ],
            ),
        )
    }

    fn create_mentioned_term() -> FatTerm {
        FatTerm::new(
            Comment::new(
                NameDescription::new("first_body_term", "first body term description"),
                &[NameDescription::new("FirstArg", "First arg's description")],
                &["test".to_string()],
            ),
            Term::new(&[], &[]),
        )
    }

    fn create_test_term() -> FatTerm {
        FatTerm::new(
            Comment::new(
                NameDescription::new("test", "test description"),
                &[NameDescription::new("FirstArg", "First arg's description")],
                &["first_related".to_string()],
            ),
            Term::new(
                &[ArgsBinding {
                    binding: vec!["fact_value".to_string()],
                }],
                &[
                    Rule {
                        head: ArgsBinding {
                            binding: vec!["FirstHeadArg".to_string()],
                        },
                        body: vec![
                            BoundTerm {
                                name: "first_body_term".to_string(),
                                arg_bindings: ArgsBinding {
                                    binding: vec!["with_some_arg".to_string()],
                                },
                            },
                            BoundTerm {
                                name: "second_body_term".to_string(),
                                arg_bindings: ArgsBinding {
                                    binding: vec!["with_some_arg2".to_string()],
                                },
                            },
                        ],
                    },
                    Rule {
                        head: ArgsBinding {
                            binding: vec!["FirstHeadArg2".to_string()],
                        },
                        body: vec![
                            BoundTerm {
                                name: "first_body_term2".to_string(),
                                arg_bindings: ArgsBinding {
                                    binding: vec!["with_some_arg".to_string()],
                                },
                            },
                            BoundTerm {
                                name: "second_body_term2".to_string(),
                                arg_bindings: ArgsBinding {
                                    binding: vec!["with_some_arg2".to_string()],
                                },
                            },
                        ],
                    },
                ],
            ),
        )
    }

    #[test]
    fn test_affected_from_changes_name_change() {
        let original = create_test_term();

        let mut with_name_change = original.clone();
        with_name_change.meta.term.name = "new_name".to_string();

        let change = Change::new(original, &[], with_name_change);
        let (mentioned, referred_by) = change.affects();
        let mut affected: Vec<String> =
            HashSet::<String>::from_iter(mentioned.into_iter().chain(referred_by))
                .into_iter()
                .collect();

        affected.sort();
        let mut expected = vec![
            "first_related".to_string(),
            "first_body_term".to_string(),
            "second_body_term".to_string(),
            "first_body_term2".to_string(),
            "second_body_term2".to_string(),
        ];

        expected.sort();
        assert_eq!(affected, expected);
    }

    #[test]
    fn test_affected_from_changes_description_change() {
        let original = create_test_term();

        let mut with_descritpion_change = original.clone();
        with_descritpion_change.meta.term.desc = "new description".to_string();

        let change = Change::new(original, &[], with_descritpion_change);
        let (mentioned, referred_by) = change.affects();
        let affected: Vec<String> =
            HashSet::<String>::from_iter(mentioned.into_iter().chain(referred_by))
                .into_iter()
                .collect();
        assert_eq!(affected.len(), 0);
    }

    #[test]
    fn test_affected_from_changes_facts_change() {
        let original = create_test_term();

        let mut with_facts_change = original.clone();
        with_facts_change.term.facts.push(ArgsBinding {
            binding: vec!["SomeArgValue".to_string()],
        });

        let change = Change::new(original, &[], with_facts_change);
        let (mentioned, referred_by) = change.affects();
        let affected: Vec<String> =
            HashSet::<String>::from_iter(mentioned.into_iter().chain(referred_by))
                .into_iter()
                .collect();

        assert_eq!(affected.len(), 0);
    }

    #[test]
    fn test_affected_from_changes_args_rename() {
        let original = create_test_term();

        let mut with_args_rename = original.clone();
        *with_args_rename.meta.args.last_mut().unwrap() =
            NameDescription::new("NewArgName", "New desc");

        let change = Change::new(original, &[], with_args_rename);
        let (mentioned, referred_by) = change.affects();
        let affected: Vec<String> =
            HashSet::<String>::from_iter(mentioned.into_iter().chain(referred_by))
                .into_iter()
                .collect();
        assert_eq!(affected.len(), 0);
    }

    #[test]
    fn test_affected_from_changes_rules_change() {
        let original = create_test_term();

        let mut with_rules_change = original.clone();
        let new_rule = Rule {
            head: ArgsBinding {
                binding: vec!["Arg".to_string()],
            },
            body: vec![BoundTerm {
                name: "new_rule_body_term".to_string(),
                arg_bindings: ArgsBinding {
                    binding: vec!["with_some_arg".to_string()],
                },
            }],
        };

        // remove the first and add another
        with_rules_change.term.rules.remove(0);
        with_rules_change.term.rules.push(new_rule);

        let change = Change::new(original, &[], with_rules_change);
        let (mentioned, referred_by) = change.affects();
        let mut affected: Vec<String> =
            HashSet::<String>::from_iter(mentioned.into_iter().chain(referred_by))
                .into_iter()
                .collect();

        affected.sort();
        let mut expected = vec!["first_body_term", "second_body_term", "new_rule_body_term"];
        expected.sort();

        assert_eq!(affected, expected);
    }

    #[test]
    fn test_affected_from_changes_arg_changes() {
        let original = create_test_term();

        let mut with_arg_change = original.clone();
        let new_arg = NameDescription::new("SomeNewArg", "With some desc");
        with_arg_change.meta.args.push(new_arg.clone());

        let change = Change::new(
            original,
            &[ArgsChange::Pushed(NameDescription::new(&new_arg.name, ""))],
            with_arg_change,
        );
        let (mentioned, referred_by) = change.affects();
        let affected: Vec<String> =
            HashSet::<String>::from_iter(mentioned.into_iter().chain(referred_by))
                .into_iter()
                .collect();
        assert_eq!(affected, vec!["first_related".to_string()]);
    }

    #[test]
    fn test_affected_from_changes_arg_and_rules_changes() {
        let original = create_test_term();

        let mut with_changes = original.clone();
        let new_rule = Rule {
            head: ArgsBinding {
                binding: vec!["Arg".to_string()],
            },
            body: vec![BoundTerm {
                name: "new_rule_body_term".to_string(),
                arg_bindings: ArgsBinding {
                    binding: vec!["with_some_arg".to_string()],
                },
            }],
        };

        // remove the first add another
        with_changes.term.rules.remove(0);
        with_changes.term.rules.push(new_rule);

        let new_arg = NameDescription::new("SomeNewArg", "With some desc");
        with_changes.meta.args.push(new_arg.clone());

        let change = Change::new(
            original,
            &[ArgsChange::Pushed(NameDescription::new(&new_arg.name, ""))],
            with_changes,
        );
        let (mentioned, referred_by) = change.affects();
        let mut affected: Vec<String> =
            HashSet::<String>::from_iter(mentioned.into_iter().chain(referred_by))
                .into_iter()
                .collect();
        affected.sort();

        let mut expected = vec![
            "first_related",
            "first_body_term",
            "second_body_term",
            "new_rule_body_term",
        ];
        expected.sort();
        assert_eq!(affected, expected);
    }

    #[test]
    fn test_apply_pushed_and_moved_arg() {
        let original = create_test_term();
        let pushed_arg = NameDescription::new("New_arg", "with some descritpion");
        let mut after_change = original.clone();
        after_change.meta.args.push(pushed_arg.clone());

        let related_term = create_related_test_term();

        let arg_change = ArgsChange::Pushed(NameDescription::new(&pushed_arg.name, ""));

        let change = Change::new(original, &[arg_change], after_change.clone());
        let result = related_term
            .apply(&change)
            .get(&related_term.meta.term.name)
            .unwrap()
            .to_owned();

        let mut expected = related_term;
        let idx = expected.term.rules[1]
            .body
            .iter_mut()
            .position(|x| x.name == "test")
            .unwrap();
        expected.term.rules[1].body[idx]
            .arg_bindings
            .binding
            .push("_".to_string());

        assert_eq!(result, expected);

        // now go for a move
        let original = after_change.clone();
        after_change.meta.args.swap(1, 0);

        let change = Change::new(original, &[ArgsChange::Moved(vec![1, 0])], after_change);
        let result = expected
            .apply(&change)
            .get(&expected.meta.term.name)
            .unwrap()
            .to_owned();

        let idx = expected.term.rules[1]
            .body
            .iter_mut()
            .position(|x| x.name == "test")
            .unwrap();
        expected.term.rules[1].body[idx]
            .arg_bindings
            .binding
            .swap(1, 0);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_applys_removed_arg() {
        let original = create_test_term();
        let mut after_change = original.clone();
        after_change.meta.args.pop();

        let related_term = create_related_test_term();

        let change = Change::new(original, &[ArgsChange::Removed(0)], after_change);
        let result = related_term
            .apply(&change)
            .get(&related_term.meta.term.name)
            .unwrap()
            .to_owned();

        let mut expected = related_term;

        let idx = expected.term.rules[1]
            .body
            .iter_mut()
            .position(|x| x.name == "test")
            .unwrap();
        expected.term.rules[1].body[idx].arg_bindings.binding.pop();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_apply_rule_change() {
        let original = create_test_term();
        let mut after_changes = original.clone();
        after_changes.term.rules[0].body.remove(0);

        let mentioned = create_mentioned_term();
        assert_eq!(mentioned.meta.referred_by, vec!["test"]);

        let change = Change::new(original, &[], after_changes);
        let result = mentioned
            .apply(&change)
            .get(&mentioned.meta.term.name)
            .unwrap()
            .to_owned();
        let mut expected = mentioned;
        expected.meta.referred_by.clear();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_apply_rename() {
        let original = create_test_term();
        let mut updated = original.clone();
        updated.meta.term.name = "new_name_who_dis".to_string();

        let mentioned = create_mentioned_term();
        assert_eq!(mentioned.meta.referred_by, vec!["test"]);
        let related = create_related_test_term();

        let change = Change::new(original.clone(), &[], updated.clone());
        let changed_mentioned = mentioned
            .apply(&change)
            .get(&mentioned.meta.term.name)
            .unwrap()
            .to_owned();
        let mut expected_changed_mentioned = mentioned;
        expected_changed_mentioned.meta.referred_by = vec![updated.meta.term.name.clone()];
        assert_eq!(changed_mentioned, expected_changed_mentioned);

        let change = Change::new(original, &[], updated.clone());
        let changed_related = related
            .apply(&change)
            .get(&related.meta.term.name)
            .unwrap()
            .to_owned();
        let mut expected_changed_related = related;
        let idx = expected_changed_related.term.rules[1]
            .body
            .iter_mut()
            .position(|x| x.name == "test")
            .unwrap();
        expected_changed_related.term.rules[1].body[idx].name = updated.meta.term.name;
        assert_eq!(changed_related, expected_changed_related);
    }
}
