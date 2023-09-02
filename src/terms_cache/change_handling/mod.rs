use std::{cell::RefCell, collections::HashSet, rc::Rc};

use its_logical::{
    changes::{
        change::{Apply, ArgsChange, Change},
        deletion::Deletion,
    },
    knowledge::{self, model::fat_term::FatTerm},
};
use tracing::debug;

use self::two_phase_commit::TwoPhaseCommit;

use super::{NamedTerm, TermHolder, TermsCache, TwoPhaseTerm};

pub(crate) mod two_phase_commit;

pub(crate) trait AutoApply {
    fn apply(&mut self, f: impl Fn(&FatTerm) -> FatTerm);
}

pub(crate) trait ConfirmationApply {
    fn push_for_confirmation(
        &mut self,
        arg_changes: &[ArgsChange],
        resulting_term: &FatTerm,
        source: &str,
    );
}

impl<T, K> TermsCache<T, K>
where
    T: NamedTerm + AutoApply,
    K: TwoPhaseTerm<Creator = T> + AutoApply + ConfirmationApply,
{
    pub(crate) fn handle_change(
        &mut self,
        knowledge_store: &impl knowledge::store::Get,
        change: &Change,
    ) {
        let (mut mentioned, referred_by) = change.affects();

        mentioned.extend(referred_by.clone());
        let all_affected: Vec<String> = mentioned.into_iter().collect();

        debug!(
            "Changes made for {}. Propagating to: {:?}",
            change.original().meta.term.name,
            all_affected
        );

        if
        /* the changes are not worthy of user confirmation */
        change.arg_changes().is_empty()
        || /* no referring term is affected */ referred_by.is_empty()
        {
            debug!("automatic propagation");
            self.apply_automatic_change(change)
        } else {
            debug!("2 phase commit propagation");
            self.apply_for_confirmation_change(knowledge_store, change);
        }
        // if there is an ongoing 2phase commit among one of `updated_term`'s newly mentioned terms,
        // all the changes in the commit need to be applied on `updated_term`
        if self.iter().any(|t| matches!(t, TermHolder::TwoPhase(_))) {
            self.repeat_ongoing_commit_changes(change);
        }
    }

    pub(crate) fn handle_deletion(
        &mut self,
        term: &FatTerm,
        knowledge_store: &impl knowledge::store::Get,
    ) {
        let changed_by_deletion = term.apply_deletion(knowledge_store);

        if term.meta.referred_by.is_empty() {
            debug!("automatic deletion");
            let update = |t: &FatTerm| -> FatTerm {
                term.apply_deletion(t)
                    .get(&t.meta.term.name)
                    .unwrap_or(t)
                    .to_owned()
            };
            for term_name in changed_by_deletion.keys() {
                if let Some(cached_term) = self.get_mut(&term_name) {
                    match cached_term {
                        TermHolder::Normal(s) => s.apply(update),
                        TermHolder::TwoPhase(s) => s.apply(update),
                    }
                }
            }
            self.remove(&term.meta.term.name);
        } else {
            debug!("deletion with confirmation");
            let deleted_two_phase_commit = self
                .promote(&term.meta.term.name)
                .expect("it must be opened as it was just deleted")
                .two_phase_commit()
                .to_owned();

            for (term_name, changed_term) in changed_by_deletion {
                if self.get(&term_name).is_none() {
                    self.push(
                        &knowledge_store
                            .get(&term_name)
                            .expect("this term has come from the knowledge store"),
                    );
                }

                let affected_by_deletion_two_phase_commit =
                    self.promote(&term_name).expect("term was just pushed");

                affected_by_deletion_two_phase_commit.push_for_confirmation(
                    &[],
                    &changed_term,
                    &term.meta.term.name,
                );
                fix_approvals(
                    affected_by_deletion_two_phase_commit.two_phase_commit(),
                    &deleted_two_phase_commit,
                )
            }
        }
    }
    fn repeat_ongoing_commit_changes(&mut self, change: &Change) {
        let previously_mentioned_terms = change.original().mentioned_terms();
        let currently_mentioned_terms = change.changed().mentioned_terms();

        let newly_mentioned_terms: HashSet<String> = currently_mentioned_terms
            .difference(&previously_mentioned_terms)
            .cloned()
            .collect();

        let mut need_to_transfer_to_commit = false;
        for term in &self.terms {
            if let super::TermHolder::TwoPhase(t) = term {
                if newly_mentioned_terms.contains(&t.name()) {
                    need_to_transfer_to_commit = true;
                    break;
                }
            }
        }
        if !need_to_transfer_to_commit {
            return;
        }

        self.promote(&change.changed().meta.term.name);

        let (mut updated_term, others): (Vec<_>, Vec<_>) = self
            .iter_mut()
            .partition(|x| &x.name() == &change.changed().meta.term.name);

        let updated_term = match updated_term.get_mut(0) {
            Some(TermHolder::TwoPhase(t)) => t,
            _ => unreachable!("updated term was just promoted"),
        };

        for term in others
            .iter()
            .filter(|x| newly_mentioned_terms.contains(&x.name()))
        {
            if let super::TermHolder::TwoPhase(mentioned_term) = term {
                let mentioned_term_change = mentioned_term.current_change();

                if let Some(with_applied_change) = updated_term
                    .term()
                    .apply(&mentioned_term_change)
                    .get(&change.changed().meta.term.name)
                {
                    updated_term.push_for_confirmation(
                        &[],
                        with_applied_change,
                        &mentioned_term.name(),
                    );
                    fix_approvals(
                        updated_term.two_phase_commit(),
                        mentioned_term.two_phase_commit(),
                    );
                }
            }
        }
    }
}

impl<T, K> TermsCache<T, K>
where
    T: NamedTerm + AutoApply,
    K: TwoPhaseTerm + AutoApply,
{
    fn apply_automatic_change(&mut self, change: &Change) {
        let update_fn = |in_term: &FatTerm| -> FatTerm {
            in_term
                .apply(change)
                .get(&in_term.meta.term.name)
                // the change might not affect the in_term so it needs to be returned as is
                .unwrap_or(in_term)
                .to_owned()
        };
        for term in &mut self.terms {
            match term {
                super::TermHolder::Normal(t) => t.apply(update_fn),
                super::TermHolder::TwoPhase(t) => t.apply(update_fn),
            }
        }
    }
}

impl<T, K> TermsCache<T, K>
where
    T: NamedTerm,
    K: TwoPhaseTerm<Creator = T> + ConfirmationApply,
{
    // applies a change that should be confirmed to all potentially
    // affected `super::TermHolder::TwoPhase` entries. Meaning that all
    fn apply_for_confirmation_change(
        &mut self,
        // the knowledge::store::Get is needed as the change might affect terms that are not yet cached in
        // the TermsCache, so they would need to be cached during this call
        knowledge_store: &impl knowledge::store::Get,
        change: &Change,
    ) -> Result<(), &'static str> {
        let all_affected = knowledge_store.apply(change);
        if all_affected
            .keys()
            .any(|affected_name| match self.get(affected_name) {
                // TODO: check if ready for change
                Some(_) => false,
                None => false,
            })
        {
            return Err("There is a term that is not ready to be included in a 2 phase commit");
        }
        let original = change.original();

        if self.get(&original.meta.term.name).is_none() {
            self.push(original);
        }

        let change_source_two_phase_commit = self
            .promote(&original.meta.term.name)
            .expect("guaranteed to be opened above")
            .two_phase_commit()
            .to_owned();

        for (name, term) in all_affected {
            if self.get(&name).is_none() {
                self.push(&term);
            }
            if let Some(two_phase) = self.promote(&name) {
                let term = two_phase.term();

                if let Some(after_change) = term.apply(change).get(&term.meta.term.name) {
                    two_phase.push_for_confirmation(
                        change.arg_changes(),
                        after_change,
                        &original.meta.term.name,
                    );
                    fix_approvals(
                        two_phase.two_phase_commit(),
                        &change_source_two_phase_commit,
                    );
                }
            };
        }
        Ok(())
    }
}

fn fix_approvals(approver: &Rc<RefCell<TwoPhaseCommit>>, waiter: &Rc<RefCell<TwoPhaseCommit>>) {
    let approve = Rc::new(RefCell::new(false));
    approver.borrow_mut().add_approval_waiter(&approve);

    waiter
        .borrow_mut()
        .wait_approval_from(&(approver.to_owned(), approve));
}

#[cfg(test)]
mod tests {
    use its_logical::knowledge::{
        model::{
            comment::{comment::Comment, name_description::NameDescription},
            term::{
                args_binding::ArgsBinding, bound_term::BoundTerm, rule::parse_rule, term::Term,
            },
        },
        store::InMemoryTerms,
    };
    use std::collections::HashMap;

    use super::*;

    // original, referring and mentioned terms prepared in the database and opened in the tabs
    fn setup() -> (Tabs, InMemoryTerms) {
        let original_term = FatTerm::new(
            Comment::new(
                NameDescription::new("original", "original description"),
                &[NameDescription::new("First_arg", "first arg description")],
                &["referring".to_string()],
            ),
            Term::new(
                &[],
                &[
                    parse_rule("original(FirstRuleArg):-mentioned(FirstRuleArg).")
                        .unwrap()
                        .1,
                ],
            ),
        );

        let referring_term = FatTerm::new(
            Comment::new(
                NameDescription::new("referring", "referring description"),
                &[NameDescription::new("First_arg", "first arg description")],
                &[],
            ),
            Term::new(
                &[],
                &[
                    parse_rule("referring(FirstRuleArg):-original(FirstRuleArg).")
                        .unwrap()
                        .1,
                ],
            ),
        );

        let mentioned_term = FatTerm::new(
            Comment::new(
                NameDescription::new("mentioned", "mentioned description"),
                &[NameDescription::new("First_arg", "first arg description")],
                &["original".to_string()],
            ),
            Term::new(&[], &[]),
        );

        let mut tabs = Tabs::default();

        tabs.push(&original_term);
        tabs.push(&referring_term);
        tabs.push(&mentioned_term);

        let databse = InMemoryTerms::new(HashMap::from([
            (original_term.meta.term.name.clone(), original_term),
            (referring_term.meta.term.name.clone(), referring_term),
            (mentioned_term.meta.term.name.clone(), mentioned_term),
        ]));

        (tabs, databse)
    }

    /** automatic changes handling */
    #[test]
    fn when_empty_args_changes_with_affected_not_new() {
        let (mut tabs, mut database) = setup();
        let original = database.get("original").unwrap();
        let mut updated = original.clone();
        updated.meta.term.name = "new_name".to_string();

        let args_changes = vec![];

        // only original tab is opened
        tabs.term_tabs.close("referring");
        tabs.term_tabs.close("mentioned");

        // should trigger automatic changes
        handle_changes(
            &mut tabs,
            &mut database,
            &original,
            &args_changes,
            updated.clone(),
        );

        assert!(!tabs.select("original"));
        assert!(database.get("original").is_none());

        assert_eq!(
            tabs.term_tabs.get("new_name").unwrap().extract_term(),
            updated
        );
        assert_eq!(database.get("new_name").unwrap(), updated);

        // should not open the related terms in tabs as this is automatic
        assert!(!tabs.select("referring"));
        assert!(!tabs.select("mentioned"));

        // these are actually not really needed in this test
        let referring = database.get("referring").unwrap();
        assert_eq!(referring.term.rules[0].body[0].name, "new_name");

        let mentioned = database.get("mentioned").unwrap();
        assert_eq!(mentioned.meta.referred_by[0], "new_name");
    }

    #[test]
    fn when_args_changes_and_no_affected_not_new() {
        let (mut tabs, mut database) = setup();
        let mut original = database.get("original").unwrap();
        original.remove_referred_by("referring");
        let mut updated = original.clone();

        let before_change_referring = database.get("referring").unwrap();
        let before_change_mentioned = database.get("mentioned").unwrap();

        // with arg change
        let args_changes = vec![change::ArgsChange::Pushed(NameDescription::new(
            "some", "arg",
        ))];

        updated.meta.args.push(NameDescription::new("some", "arg"));
        updated.term.rules[0].head.binding.push("_".to_string());

        // only original tab is opened
        tabs.term_tabs.close("referring");
        tabs.term_tabs.close("mentioned");

        // should trigger automatic changes
        handle_changes(
            &mut tabs,
            &mut database,
            &original,
            &args_changes,
            updated.clone(),
        );

        assert_eq!(
            tabs.term_tabs.get("original").unwrap().extract_term(),
            updated
        );
        assert_eq!(database.get("original").unwrap(), updated);

        assert_eq!(database.get("referring").unwrap(), before_change_referring);
        assert_eq!(database.get("mentioned").unwrap(), before_change_mentioned);

        // should not open the related terms in tabs as this is automatic
        assert!(!tabs.select("referring"));
        assert!(!tabs.select("mentioned"));
    }

    #[test]
    fn when_args_changes_and_affected_and_new() {
        let (mut tabs, mut database) = setup();
        let mut original = database.get("original").unwrap();
        let mut updated = original.clone();
        // a newly created term can't be referred by any other terms
        updated.remove_referred_by("referring");

        // the arg is essentially a new arg
        let args_changes = vec![change::ArgsChange::Pushed(updated.meta.args[0].clone())];

        // simulate starting with a blank term
        original = FatTerm::default();
        database.delete("original");
        tabs.term_tabs.close("original");
        tabs.push(&original);
        let mut mentioned = database.get("mentioned").unwrap();
        mentioned.remove_referred_by("original");
        database.put("mentioned", mentioned).unwrap();

        let before_change_referring = database.get("referring").unwrap();

        // only original tab is opened
        tabs.term_tabs.close("referring");
        tabs.term_tabs.close("mentioned");

        // should trigger automatic changes
        handle_changes(
            &mut tabs,
            &mut database,
            &original,
            &args_changes,
            updated.clone(),
        );

        assert_eq!(
            tabs.term_tabs.get("original").unwrap().extract_term(),
            updated
        );
        assert_eq!(database.get("original").unwrap(), updated);

        assert_eq!(database.get("referring").unwrap(), before_change_referring);
        assert_eq!(
            database.get("mentioned").unwrap().meta.referred_by,
            vec!["original"]
        );

        // should not open the related terms in tabs as this is automatic
        assert!(!tabs.select("referring"));
        assert!(!tabs.select("mentioned"));
    }

    #[test]
    fn when_args_changes_and_affected_and_not_new() {
        let (mut tabs, mut database) = setup();
        let original = database.get("original").unwrap();
        let mut updated = original.clone();

        let before_change_referring = database.get("referring").unwrap();
        let before_change_mentioned = database.get("mentioned").unwrap();

        let mut newly_mentioned = before_change_mentioned.clone();
        newly_mentioned.meta.term.name = "newly_mentioned".to_string();
        database
            .put("newly_mentioned", newly_mentioned.clone())
            .unwrap();

        // with arg change
        let args_changes = vec![change::ArgsChange::Pushed(NameDescription::new(
            "new_arg",
            "description",
        ))];

        updated.meta.args[0] = NameDescription::new("new_arg", "description");
        updated.term.rules[0].head.binding.push("_".to_string());
        updated.term.rules[0].body.push(BoundTerm {
            name: "newly_mentioned".to_string(),
            arg_bindings: ArgsBinding {
                binding: vec!["woah".to_string()],
            },
        });

        // only original tab is opened
        tabs.term_tabs.close("referring");
        tabs.term_tabs.close("mentioned");

        // should trigger with_confirmation changes
        handle_changes(
            &mut tabs,
            &mut database,
            &original,
            &args_changes,
            updated.clone(),
        );

        assert_eq!(database.get("original").unwrap(), original);
        assert_eq!(database.get("referring").unwrap(), before_change_referring);
        assert_eq!(database.get("mentioned").unwrap(), before_change_mentioned);
        assert_eq!(database.get("newly_mentioned").unwrap(), newly_mentioned);

        // there is no actual change to mentioned so it doesn't need to be opened
        assert!(!tabs.select("mentioned"));

        {
            // 2-phase-commit must be setup
            assert!(tabs.commit_tabs.is_some());
            let commit_tabs = tabs.commit_tabs.as_ref().unwrap();
            // referring needs to confirm the change so it must be opened
            let referring_tab = commit_tabs.tabs.get("referring");
            assert!(referring_tab.is_some());
            let referring_tab = referring_tab.unwrap();

            // there's a newly mentioned term that will be changed due to this change
            let newly_mentioned_tab = commit_tabs.tabs.get("newly_mentioned");
            assert!(newly_mentioned_tab.is_some());
            let newly_mentioned_tab = newly_mentioned_tab.unwrap();

            let original_tab = commit_tabs.tabs.get("original").unwrap();

            let mut original_awaiting_for: Vec<String> =
                original_tab.borrow().waiting_for().collect();
            original_awaiting_for.sort();
            let mut expected_awaitng_for = vec!["referring", "newly_mentioned"];
            expected_awaitng_for.sort();
            assert_eq!(original_awaiting_for, expected_awaitng_for);
            assert!(!original_tab.borrow().is_being_waited());

            let check_approvers = |approver_tab: &Rc<RefCell<TwoPhaseCommit>>| {
                let approver_two_phase_commit = approver_tab.borrow();

                let approver_awaiting_for: Vec<String> =
                    approver_two_phase_commit.waiting_for().collect();

                assert!(approver_awaiting_for.is_empty());
                assert!(approver_two_phase_commit.is_being_waited());
            };

            check_approvers(referring_tab);
            check_approvers(newly_mentioned_tab);
        }

        // now test what happpens when the newly mentioned is changed while the 2-phase-commit is
        // in process
        let args_changes = vec![change::ArgsChange::Pushed(NameDescription::new(
            "new_arg",
            "description",
        ))];
        let commit_tabs = tabs.commit_tabs.as_ref().unwrap();
        let newly_mentioned = commit_tabs
            .tabs
            .get("newly_mentioned")
            .unwrap()
            .borrow()
            .term
            .extract_term();
        let mut updated = newly_mentioned.clone();

        updated
            .meta
            .args
            .push(NameDescription::new("new_arg", "description"));

        // should trigger with_confirmation changes and wait for approval from "original"
        handle_changes(
            &mut tabs,
            &mut database,
            &newly_mentioned,
            &args_changes,
            updated,
        );

        {
            let commit_tabs = tabs.commit_tabs.unwrap();
            // referring needs to confirm the change so it must be opened
            let referring_tab = commit_tabs.tabs.get("referring");
            assert!(referring_tab.is_some());
            let referring_tab = referring_tab.unwrap();

            let newly_mentioned_tab = commit_tabs.tabs.get("newly_mentioned");
            assert!(newly_mentioned_tab.is_some());
            let newly_mentioned_tab = newly_mentioned_tab.unwrap();

            let original_tab = commit_tabs.tabs.get("original").unwrap();

            let mut original_awaiting_for: Vec<String> =
                original_tab.borrow().waiting_for().collect();

            original_awaiting_for.sort();
            let mut expected_awaitng_for = vec!["referring", "newly_mentioned"];
            expected_awaitng_for.sort();
            assert_eq!(original_awaiting_for, expected_awaitng_for);
            // because of the change in newly_mentioned
            assert!(original_tab.borrow().is_being_waited());

            assert!(referring_tab.borrow().is_being_waited());
            assert!(newly_mentioned_tab.borrow().is_being_waited());

            let referring_waiting_for: Vec<String> = referring_tab.borrow().waiting_for().collect();
            assert!(referring_waiting_for.is_empty());

            let newly_mentioned_waiting_for: Vec<String> =
                newly_mentioned_tab.borrow().waiting_for().collect();
            assert_eq!(newly_mentioned_waiting_for, &["original"]);
        }
    }
}
