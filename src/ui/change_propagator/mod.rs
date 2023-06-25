use std::collections::HashSet;

use tracing::debug;

use crate::model::{
    comment::name_description::NameDescription, fat_term::FatTerm, term::args_binding::ArgsBinding,
};

use super::widgets::{
    drag_and_drop, term_screen::term_screen_pit::TermChange, term_screen::Change,
};

pub(crate) mod terms_filter;

pub(crate) trait Terms {
    fn get(&self, term_name: &str) -> Option<FatTerm>;
}

pub(crate) fn need_confirmation(changes: &Change) -> bool {
    match changes {
        Change::Changes(changes, _, updated_term) => {
            for change in changes {
                if let TermChange::ArgChanges(arg_changes) = change {
                    if updated_term.meta.referred_by.len() > 0 {
                        for arg_change in arg_changes {
                            if let drag_and_drop::Change::Pushed(_)
                            | drag_and_drop::Change::Removed(_, _) = arg_change
                            {
                                // currently only a new or removed argument triggers user
                                // intervention - all other changes can be applied
                                // automaticallly
                                return true;
                            }
                        }
                    }
                }
            }
            return false;
        }
        Change::Deleted(_) => {
            return true;
        }
    }
}

pub(crate) fn get_affected(original: &FatTerm, changes: &Change) -> Vec<String> {
    let mut affected_terms = vec![];
    match changes {
        Change::Changes(changes, original_name, updated_term) => {
            let mut include_referred_by = false;
            let mut include_mentioned = false;

            if *original_name != updated_term.meta.term.name {
                include_referred_by = true;
                include_mentioned = true;
            }

            for change in changes {
                match change {
                    TermChange::DescriptionChange
                    | TermChange::FactsChange
                    | TermChange::ArgRename => {
                        debug!("internal changes");
                    }
                    TermChange::ArgChanges(_) => {
                        include_referred_by = true;
                    }
                    TermChange::RuleChanges => {
                        if include_mentioned {
                            // all mentioned are already included so there's no need to figure out
                            // new and old
                        } else {
                            let (mut new, mut removed) =
                                changes_in_mentioned_terms(original, &updated_term);

                            affected_terms.append(&mut new);
                            affected_terms.append(&mut removed);
                        }
                    }
                };
            }
            if include_referred_by {
                affected_terms.append(&mut updated_term.meta.referred_by.clone());
            }
            if include_mentioned {
                let old_mentioned = get_mentioned_terms(original);
                let current_mentioned = get_mentioned_terms(updated_term);

                affected_terms.append(
                    &mut old_mentioned
                        .union(&current_mentioned)
                        .into_iter()
                        .cloned()
                        .collect(),
                );
            }
        }
        Change::Deleted(_) => {
            // need to remove the term from all the terms' "referred by" field
            affected_terms.append(&mut get_mentioned_terms(original).into_iter().collect());
            // need to remove the term from all the terms' rules that refer to it
            affected_terms.append(&mut original.meta.referred_by.clone());
        }
    }
    affected_terms
}

// the term that the changes were done on is also put through the filter
pub(crate) fn apply_changes(
    changes: &Change,
    original: &FatTerm,
    terms: &mut impl terms_filter::TermsFilter,
) {
    match changes {
        Change::Changes(changes, original_name, updated_term) => {
            terms.put(&original_name, &updated_term.clone());
            for change in changes {
                match change {
                    TermChange::DescriptionChange
                    | TermChange::FactsChange
                    | TermChange::ArgRename => {
                        debug!("internal changes");
                    }
                    TermChange::ArgChanges(arg_changes) => {
                        for referred_by_term_name in &updated_term.meta.referred_by {
                            if let Some(term) = terms.get(referred_by_term_name) {
                                apply_arg_changes(term, &original_name, arg_changes.iter());
                            }
                        }
                        if let Some(changed_term) = terms.get(&original_name) {
                            apply_head_arg_changes(changed_term, arg_changes.iter());
                        }
                    }
                    TermChange::RuleChanges => {
                        let (new, removed) = changes_in_mentioned_terms(original, &updated_term);
                        for term_name_with_removed_mention in &removed {
                            if let Some(term) = terms.get(term_name_with_removed_mention) {
                                term.remove_referred_by(&original_name);
                            }
                        }

                        for term_name_with_new_mention in &new {
                            if let Some(term) = terms.get(&term_name_with_new_mention) {
                                term.add_referred_by(&original_name);
                            }
                        }
                    }
                };
            }
            // once all externally propagated changes are applied with the original name,
            // the potential name change is addressed
            if original_name != &updated_term.meta.term.name {
                for rule in updated_term.term.rules.iter() {
                    for body_term in &rule.body {
                        if let Some(term) = terms.get(&body_term.name) {
                            term.rename_referred_by(&original_name, &updated_term.meta.term.name);
                        }
                    }
                }

                for referred_by_term_name in &updated_term.meta.referred_by {
                    if let Some(term) = terms.get(referred_by_term_name) {
                        for rule in &mut term.term.rules {
                            for body_term in &mut rule.body {
                                if &body_term.name == original_name {
                                    body_term.name = updated_term.meta.term.name.clone();
                                }
                            }
                        }
                    }
                }
            }
        }
        Change::Deleted(term_name) => {
            for rule in original.term.rules.iter() {
                for body_term in &rule.body {
                    if let Some(term) = terms.get(&body_term.name) {
                        term.remove_referred_by(term_name);
                    }
                }
            }

            for referred_by_term_name in &original.meta.referred_by {
                if let Some(term) = terms.get(&referred_by_term_name) {
                    for rule in &mut term.term.rules {
                        let before_removal_body_term_count = rule.body.len();
                        rule.body.retain(|body_term| body_term.name != *term_name);

                        /*
                        if rule.body.len() < before_removal_body_term_count {
                            // A confirmation is needed only if actual removing was done. There is the
                            // case when the user has already confirmed the deletion and this code path
                            // does not remove anything.
                            needs_confirmation = true;
                        }
                        */
                    }
                }
            }
        }
    }
}

fn get_mentioned_terms(term: &FatTerm) -> HashSet<String> {
    let mut mentioned_terms = HashSet::<String>::new();

    for rule in term.term.rules.iter() {
        for body_term in &rule.body {
            mentioned_terms.insert(body_term.name.clone());
        }
    }
    mentioned_terms
}

fn changes_in_mentioned_terms(
    original_term: &FatTerm,
    term: &FatTerm,
) -> (Vec<String>, Vec<String>) {
    let old_related_terms = get_mentioned_terms(original_term);
    let related_terms = get_mentioned_terms(term);

    return (
        related_terms
            .difference(&old_related_terms)
            .into_iter()
            .cloned()
            .collect(),
        old_related_terms
            .difference(&related_terms)
            .into_iter()
            .cloned()
            .collect(),
    );
}

fn apply_arg_changes<'a>(
    term: &mut FatTerm,
    term_with_arg_change: &str,
    changes: impl Iterator<Item = &'a drag_and_drop::Change<NameDescription>>,
) {
    for change in changes {
        for rule in &mut term.term.rules {
            for body_term in &mut rule.body {
                if body_term.name == term_with_arg_change {
                    apply_binding_change(change, &mut body_term.arg_bindings);
                }
            }
        }
    }
}

fn apply_binding_change(
    change: &drag_and_drop::Change<NameDescription>,
    binding: &mut ArgsBinding,
) -> Option<String> {
    match change {
        drag_and_drop::Change::Pushed(_) => {
            binding.binding.push("_".to_string());
        }
        drag_and_drop::Change::Moved(moves) => {
            let mut projected = vec!["".to_string(); binding.binding.len()];

            for (idx, old_idx) in moves.iter().enumerate() {
                projected[idx] = binding.binding[*old_idx].clone();
            }
            binding.binding = projected;
        }
        drag_and_drop::Change::Removed(removed_idx, _) => {
            return Some(binding.binding.remove(*removed_idx));
        }
    }
    None
}

fn apply_head_arg_changes<'a>(
    term: &mut FatTerm,
    changes: impl Iterator<Item = &'a drag_and_drop::Change<NameDescription>>,
) {
    for change in changes {
        for rule in &mut term.term.rules {
            let removed_arg = apply_binding_change(change, &mut rule.arg_bindings);

            if let Some(removed_arg) = removed_arg {
                for body_term in &mut rule.body {
                    for bound_arg in &mut body_term.arg_bindings.binding {
                        if bound_arg == &removed_arg {
                            *bound_arg = "_".to_string();
                        }
                    }
                }
            }
        }

        for fact in &mut term.term.facts {
            apply_binding_change(change, fact);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{terms_filter::TermsFilter, *};
    use crate::model::{
        comment::{comment::Comment, name_description::NameDescription},
        fat_term::FatTerm,
        term::{args_binding::ArgsBinding, bound_term::BoundTerm, rule::Rule, term::Term},
    };

    fn create_related_test_term() -> FatTerm {
        FatTerm::new(
            Comment::new(
                NameDescription::new("first_related", "related description"),
                vec![NameDescription::new("FirstArg", "First arg's description")],
                vec![],
            ),
            Term::new(
                vec![ArgsBinding {
                    binding: vec!["fact_value".to_string()],
                }],
                vec![
                    Rule {
                        arg_bindings: ArgsBinding {
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
                        arg_bindings: ArgsBinding {
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
                vec![NameDescription::new("FirstArg", "First arg's description")],
                vec!["test".to_string()],
            ),
            Term::new(vec![], vec![]),
        )
    }

    fn create_test_term() -> FatTerm {
        FatTerm::new(
            Comment::new(
                NameDescription::new("test", "test description"),
                vec![NameDescription::new("FirstArg", "First arg's description")],
                vec!["first_related".to_string()],
            ),
            Term::new(
                vec![ArgsBinding {
                    binding: vec!["fact_value".to_string()],
                }],
                vec![
                    Rule {
                        arg_bindings: ArgsBinding {
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
                        arg_bindings: ArgsBinding {
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
    fn test_get_affected_changes_name_change() {
        let original = create_test_term();

        let mut with_name_change = original.clone();
        with_name_change.meta.term.name = "new_name".to_string();

        let mut relevant = get_affected(
            &original.clone(),
            &Change::Changes(vec![], "test".to_string(), with_name_change),
        );
        relevant.sort();
        let mut expected = vec![
            "first_related".to_string(),
            "first_body_term".to_string(),
            "second_body_term".to_string(),
            "first_body_term2".to_string(),
            "second_body_term2".to_string(),
        ];

        expected.sort();
        assert_eq!(relevant, expected);
    }

    #[test]
    fn test_get_affected_changes_description_change() {
        let original = create_test_term();

        let mut with_descritpion_change = original.clone();
        with_descritpion_change.meta.term.desc = "new description".to_string();

        let relevant = get_affected(
            &original.clone(),
            &Change::Changes(
                vec![TermChange::DescriptionChange],
                "test".to_string(),
                with_descritpion_change,
            ),
        );
        assert_eq!(relevant.len(), 0);
    }

    #[test]
    fn test_get_affected_changes_facts_change() {
        let original = create_test_term();

        let mut with_facts_change = original.clone();
        with_facts_change.term.facts.push(ArgsBinding {
            binding: vec!["SomeArgValue".to_string()],
        });

        let relevant = get_affected(
            &original.clone(),
            &Change::Changes(
                vec![TermChange::FactsChange],
                "test".to_string(),
                with_facts_change,
            ),
        );
        assert_eq!(relevant.len(), 0);
    }

    #[test]
    fn test_get_affected_changes_args_rename() {
        let original = create_test_term();

        let mut with_args_rename = original.clone();
        *with_args_rename.meta.args.last_mut().unwrap() =
            NameDescription::new("NewArgName", "New desc");

        let relevant = get_affected(
            &original.clone(),
            &Change::Changes(
                vec![TermChange::ArgRename],
                "test".to_string(),
                with_args_rename,
            ),
        );
        assert_eq!(relevant.len(), 0);
    }

    #[test]
    fn test_get_affected_changes_rules_change() {
        let original = create_test_term();

        let mut with_rules_change = original.clone();
        let new_rule = Rule {
            arg_bindings: ArgsBinding {
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
        with_rules_change.term.rules.remove(0);
        with_rules_change.term.rules.push(new_rule.clone());

        let mut relevant = get_affected(
            &original.clone(),
            &Change::Changes(
                vec![TermChange::RuleChanges],
                "test".to_string(),
                with_rules_change,
            ),
        );
        relevant.sort();
        let mut expected = vec!["first_body_term", "second_body_term", "new_rule_body_term"];
        expected.sort();

        assert_eq!(relevant, expected);
    }

    #[test]
    fn test_get_affected_changes_arg_changes() {
        let original = create_test_term();

        let mut with_arg_change = original.clone();
        let new_arg = NameDescription::new("SomeNewArg", "With some desc");
        with_arg_change.meta.args.push(new_arg.clone());

        let relevant = get_affected(
            &original.clone(),
            &Change::Changes(
                vec![TermChange::ArgChanges(vec![drag_and_drop::Change::Pushed(
                    new_arg,
                )])],
                "test".to_string(),
                with_arg_change,
            ),
        );
        assert_eq!(relevant, vec!["first_related".to_string()]);
    }

    #[test]
    fn test_get_affected_changes_arg_and_rules_changes() {
        let original = create_test_term();

        let mut with_changes = original.clone();
        let new_rule = Rule {
            arg_bindings: ArgsBinding {
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
        with_changes.term.rules.push(new_rule.clone());

        let new_arg = NameDescription::new("SomeNewArg", "With some desc");
        with_changes.meta.args.push(new_arg.clone());

        let mut relevant = get_affected(
            &original.clone(),
            &Change::Changes(
                vec![
                    TermChange::ArgChanges(vec![drag_and_drop::Change::Pushed(new_arg)]),
                    TermChange::RuleChanges,
                ],
                "test".to_string(),
                with_changes,
            ),
        );
        relevant.sort();

        let mut expected = vec![
            "first_related",
            "first_body_term",
            "second_body_term",
            "new_rule_body_term",
        ];
        expected.sort();
        assert_eq!(relevant, expected);
    }

    /*apply_changes tests*/

    struct FakeTermsFilter {
        terms: HashMap<String, FatTerm>,
    }

    impl FakeTermsFilter {
        fn new(term: &FatTerm) -> Self {
            Self {
                terms: HashMap::from([(term.meta.term.name.clone(), term.to_owned())]),
            }
        }
    }
    impl TermsFilter for FakeTermsFilter {
        fn get<'a>(&'a mut self, name: &str) -> Option<&'a mut FatTerm> {
            self.terms.get_mut(name)
        }

        fn put(&mut self, name: &str, term: &FatTerm) {
            self.terms.insert(name.to_string(), term.to_owned());
        }

        fn all_terms(self) -> HashMap<String, FatTerm> {
            self.terms
        }
    }

    #[test]
    fn test_apply_changes_pushed_and_moved_arg() {
        let original = create_test_term();
        let pushed_arg = NameDescription::new("New_arg", "with some descritpion");
        let mut after_change = original.clone();
        after_change.meta.args.push(pushed_arg.clone());

        let related_term = create_related_test_term();

        // Define changes for ArgChanges
        let arg_change = drag_and_drop::Change::Pushed(pushed_arg.clone());
        let changes = Change::Changes(
            vec![TermChange::ArgChanges(vec![arg_change])],
            "test".to_owned(),
            after_change.clone(),
        );

        let mut filter = FakeTermsFilter::new(&related_term);
        apply_changes(&changes, &original, &mut filter);
        let result = filter.all_terms();

        // the original term's facts and rules' heads are also modified
        let mut after_change = result.get(&original.meta.term.name).unwrap().to_owned();
        let result = result.get(&related_term.meta.term.name).unwrap().to_owned();

        let mut expected = related_term.clone();

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
        after_change.meta.args.swap(1, 0);
        let changes = Change::Changes(
            vec![TermChange::ArgChanges(vec![drag_and_drop::Change::Moved(
                vec![1, 0],
            )])],
            "test".to_owned(),
            after_change.clone(),
        );

        let mut filter = FakeTermsFilter::new(&expected);
        apply_changes(&changes, &after_change, &mut filter);
        let result = filter
            .all_terms()
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
    fn test_apply_changes_removed_arg() {
        let original = create_test_term();
        let mut after_change = original.clone();
        after_change.meta.args.pop();

        let related_term = create_related_test_term();

        let arg_change = drag_and_drop::Change::Removed(0, NameDescription::new("what", "ever"));
        let changes = Change::Changes(
            vec![TermChange::ArgChanges(vec![arg_change])],
            "test".to_owned(),
            after_change,
        );

        let mut filter = FakeTermsFilter::new(&related_term);
        apply_changes(&changes, &original, &mut filter);
        let result = filter
            .all_terms()
            .get(&related_term.meta.term.name)
            .unwrap()
            .to_owned();
        let mut expected = related_term.clone();

        let idx = expected.term.rules[1]
            .body
            .iter_mut()
            .position(|x| x.name == "test")
            .unwrap();
        expected.term.rules[1].body[idx].arg_bindings.binding.pop();

        // Assert the result is the updated term
        assert_eq!(result, expected);
    }

    #[test]
    fn test_apply_changes_rule_change() {
        let original = create_test_term();
        let mut after_changes = original.clone();
        after_changes.term.rules[0].body.remove(0);

        let mentioned = create_mentioned_term();
        assert_eq!(mentioned.meta.referred_by, vec!["test"]);

        // Define changes for RuleChanges
        let changes = Change::Changes(
            vec![TermChange::RuleChanges],
            "test".to_owned(),
            after_changes,
        );

        let mut filter = FakeTermsFilter::new(&mentioned);
        apply_changes(&changes, &original, &mut filter);
        let result = filter
            .all_terms()
            .get(&mentioned.meta.term.name)
            .unwrap()
            .to_owned();
        let mut expected = mentioned;
        expected.meta.referred_by.clear();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_apply_changes_rename() {
        let original = create_test_term();
        let mut updated = original.clone();
        updated.meta.term.name = "new_name_who_dis".to_string();

        let mentioned = create_mentioned_term();
        assert_eq!(mentioned.meta.referred_by, vec!["test"]);
        let related = create_related_test_term();

        // Define changes for renaming
        let changes = Change::Changes(vec![], "test".to_owned(), updated.clone());

        let mut filter = FakeTermsFilter::new(&mentioned);
        apply_changes(&changes, &original, &mut filter);
        let changed_mentioned = filter
            .all_terms()
            .get(&mentioned.meta.term.name)
            .unwrap()
            .to_owned();
        let mut expected_changed_mentioned = mentioned;
        expected_changed_mentioned.meta.referred_by = vec![updated.meta.term.name.clone()];
        assert_eq!(changed_mentioned, expected_changed_mentioned);

        let mut filter = FakeTermsFilter::new(&related);
        apply_changes(&changes, &original, &mut filter);
        let changed_related = filter
            .all_terms()
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
