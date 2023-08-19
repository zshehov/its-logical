use std::collections::HashMap;

use crate::knowledge;
use crate::knowledge::model::{
    comment::name_description::NameDescription, fat_term::FatTerm, term::args_binding::ArgsBinding,
};

use self::terms_cache::TermsCache;

mod terms_cache;

pub struct Change {
    original: FatTerm,
    args_changes: Vec<ArgsChange>,
    changed: FatTerm,
}

pub enum TermChange {
    Rename,
    DescriptionChange,
    FactsChange,
    ArgRename,
    ArgChanges(Vec<ArgsChange>),
    RuleChanges,
}

#[derive(Clone, Debug)]
pub enum ArgsChange {
    Pushed(NameDescription),
    Moved(Vec<usize>),
    Removed(usize),
}

impl ArgsChange {
    pub fn apply(&self, binding: &mut ArgsBinding) -> Option<String> {
        match self {
            ArgsChange::Pushed(_) => binding.binding.push("_".to_string()),
            ArgsChange::Moved(moves) => {
                let mut projected = vec!["".to_string(); binding.binding.len()];

                for (idx, old_idx) in moves.iter().enumerate() {
                    projected[idx] = binding.binding[*old_idx].clone();
                }
                binding.binding = projected;
            }
            ArgsChange::Removed(removed_idx) => return Some(binding.binding.remove(*removed_idx)),
        }
        None
    }
}

pub trait Deletion {
    fn deletion_affects(&self) -> Vec<String>;
    fn apply_deletion(&self, terms: &impl knowledge::store::Get) -> HashMap<String, FatTerm>;
}

impl Deletion for FatTerm {
    fn deletion_affects(&self) -> Vec<String> {
        let mut affected_terms = vec![];
        // need to remove the term from all the terms' "referred by" field
        affected_terms.append(&mut self.mentioned_terms().into_iter().collect());
        // need to remove the term from all the terms' rules that refer to it
        affected_terms.append(&mut self.meta.referred_by.clone());
        affected_terms
    }

    fn apply_deletion(&self, terms: &impl knowledge::store::Get) -> HashMap<String, FatTerm> {
        let mut terms_cache = TermsCache::new(terms);
        for rule in self.term.rules.iter() {
            for body_term in &rule.body {
                if let Some(term) = terms_cache.get(&body_term.name) {
                    term.remove_referred_by(&self.meta.term.name);
                }
            }
        }

        for referred_by_term_name in &self.meta.referred_by {
            if let Some(term) = terms_cache.get(referred_by_term_name) {
                for rule in &mut term.term.rules {
                    rule.body
                        .retain(|body_term| body_term.name != self.meta.term.name);
                }
                term.term.rules.retain(|rule| !rule.body.is_empty());
            }
        }
        terms_cache.all_terms()
    }
}

impl Change {
    pub fn new(original: FatTerm, args_changes: &[ArgsChange], changed: FatTerm) -> Self {
        Self {
            original,
            args_changes: args_changes.to_vec(),
            changed,
        }
    }

    pub fn affects(&self) -> (Vec<String>, Vec<String>) {
        let mut mentioned = vec![];
        let mut referred_by = vec![];
        let mut include_referred_by = false;
        let mut include_mentioned = false;

        if &self.original.meta.term.name != &self.changed.meta.term.name {
            include_referred_by = true;
            include_mentioned = true;
        }

        if include_mentioned {
            // all mentioned are already included so there's no need to figure out
            // new and old
        } else {
            let (mut new, mut removed) = self.changes_in_mentioned_terms();

            mentioned.append(&mut new);
            mentioned.append(&mut removed);
        }

        if !self.args_changes.is_empty() {
            include_referred_by = true;
        }

        if include_referred_by {
            referred_by.append(&mut self.changed.meta.referred_by.clone());
        }
        if include_mentioned {
            let old_mentioned = self.original.mentioned_terms();
            let current_mentioned = self.changed.mentioned_terms();

            mentioned.append(&mut old_mentioned.union(&current_mentioned).cloned().collect());
        }
        (mentioned, referred_by)
    }

    pub fn apply(&self, terms: &impl knowledge::store::Get) -> HashMap<String, FatTerm> {
        let mut terms_cache = TermsCache::new(terms);

        if !self.args_changes.is_empty() {
            for referred_by_term_name in &self.changed.meta.referred_by {
                if let Some(term) = terms_cache.get(referred_by_term_name) {
                    self.apply_args_changes(term);
                }
            }
        }

        let (new, removed) = self.changes_in_mentioned_terms();
        for term_name_with_removed_mention in &removed {
            if let Some(term) = terms_cache.get(term_name_with_removed_mention) {
                term.remove_referred_by(&self.original.meta.term.name);
            }
        }

        for term_name_with_new_mention in &new {
            if let Some(term) = terms_cache.get(term_name_with_new_mention) {
                term.add_referred_by(&self.original.meta.term.name);
            }
        }
        // once all externally propagated changes are applied with the original name,
        // the potential name change is addressed
        if &self.original.meta.term.name != &self.changed.meta.term.name {
            for rule in self.changed.term.rules.iter() {
                for body_term in &rule.body {
                    if let Some(term) = terms_cache.get(&body_term.name) {
                        term.rename_referred_by(
                            &self.original.meta.term.name,
                            &self.changed.meta.term.name,
                        );
                    }
                }
            }

            for referred_by_term_name in &self.changed.meta.referred_by {
                if let Some(term) = terms_cache.get(referred_by_term_name) {
                    for rule in &mut term.term.rules {
                        for body_term in &mut rule.body {
                            if &body_term.name == &self.original.meta.term.name {
                                body_term.name = self.changed.meta.term.name.clone();
                            }
                        }
                    }
                }
            }
        }
        terms_cache.all_terms()
    }

    fn changes_in_mentioned_terms(&self) -> (Vec<String>, Vec<String>) {
        let old_related_terms = self.original.mentioned_terms();
        let related_terms = self.changed.mentioned_terms();

        return (
            related_terms
                .difference(&old_related_terms)
                .cloned()
                .collect(),
            old_related_terms
                .difference(&related_terms)
                .cloned()
                .collect(),
        );
    }

    fn apply_args_changes(&self, target_term: &mut FatTerm) {
        for rule in &mut target_term.term.rules {
            for body_term in &mut rule.body {
                if &body_term.name == &self.original.meta.term.name {
                    for change in &self.args_changes {
                        change.apply(&mut body_term.arg_bindings);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::changes::{ArgsChange, Change};
    use crate::knowledge::{
        model::{
            comment::{comment::Comment, name_description::NameDescription},
            fat_term::FatTerm,
            term::{args_binding::ArgsBinding, bound_term::BoundTerm, rule::Rule, term::Term},
        },
        store::Get,
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

    struct FakeTermHolder {
        term: FatTerm,
    }

    impl FakeTermHolder {
        fn new(term: &FatTerm) -> Self {
            Self { term: term.clone() }
        }
    }
    impl Get for FakeTermHolder {
        fn get(&self, term_name: &str) -> Option<FatTerm> {
            if term_name == self.term.meta.term.name {
                Some(self.term.clone())
            } else {
                None
            }
        }
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
        let result = change
            .apply(&FakeTermHolder::new(&related_term))
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
        let result = change
            .apply(&FakeTermHolder::new(&expected))
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
        let result = change
            .apply(&FakeTermHolder::new(&related_term))
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
        let result = change
            .apply(&FakeTermHolder::new(&mentioned))
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
        let changed_mentioned = change
            .apply(&FakeTermHolder::new(&mentioned))
            .get(&mentioned.meta.term.name)
            .unwrap()
            .to_owned();
        let mut expected_changed_mentioned = mentioned;
        expected_changed_mentioned.meta.referred_by = vec![updated.meta.term.name.clone()];
        assert_eq!(changed_mentioned, expected_changed_mentioned);

        let change = Change::new(original.clone(), &[], updated.clone());
        let changed_related = change
            .apply(&FakeTermHolder::new(&related))
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
