use std::{cell::RefCell, collections::HashSet, rc::Rc};

use tracing::debug;

use crate::{
    changes,
    model::fat_term::FatTerm,
    term_knowledge_base::{DeleteKnowledgeBase, GetKnowledgeBase, PutKnowledgeBase},
};

use self::with_confirmation::loaded::TabsWithLoading;

use super::widgets::{
    tabs::{commit_tabs::two_phase_commit::TwoPhaseCommit, term_tabs::TermTabs, Tabs},
    term_screen::{term_screen_pit::TermChange, TermScreen},
};

mod automatic;
mod with_confirmation;

pub(crate) fn handle_changes(
    tabs: &mut Tabs,
    terms: &mut (impl GetKnowledgeBase + PutKnowledgeBase),
    original_term: &FatTerm,
    term_changes: &[TermChange],
    updated_term: FatTerm,
) {
    let original_name = original_term.meta.term.name.clone();
    // only argument changes are tough and need special care
    let arg_changes = term_changes
        .iter()
        .find_map(|change| {
            if let TermChange::ArgChanges(arg_changes) = change {
                return Some(arg_changes.iter().map(|x| x.into()).collect());
            }
            None
        })
        .unwrap_or(vec![]);

    let (mentioned, referred_by) =
        changes::propagation::affected_from_changes(original_term, &updated_term, &arg_changes);

    let mut affected: HashSet<String> = HashSet::from_iter(mentioned);
    affected.extend(referred_by.clone());
    let affected: Vec<String> = affected.into_iter().collect();

    debug!(
        "Changes made for {}. Propagating to: {:?}",
        original_name, affected
    );

    if
    /* the changes are not worthy of user confirmation */
    arg_changes.is_empty()
        || /* no referring term is affected */ referred_by.is_empty()
    {
        debug!("automatic propagation");
        automatic::propagate(terms, tabs, original_term, &updated_term, &affected);
    } else {
        debug!("2 phase commit propagation");
        with_confirmation::propagate(
            TabsWithLoading::new(tabs, terms),
            original_term,
            &arg_changes,
            &updated_term,
            &affected,
        );
    }
    // if there is an ongoing 2phase commit among one of `updated_term`'s newly mentioned terms,
    // all the changes in the commit need to be applied on `updated_term`
    if let Some(commit_tabs) = &mut tabs.commit_tabs {
        repeat_ongoing_commit_changes(
            &mut tabs.term_tabs,
            &mut commit_tabs.tabs,
            original_term,
            updated_term,
        );
    }
}

pub(crate) fn handle_deletion(
    tabs: &mut Tabs,
    terms: &mut (impl GetKnowledgeBase + PutKnowledgeBase + DeleteKnowledgeBase),
    original_term: &FatTerm,
) {
    if !original_term.meta.referred_by.is_empty() {
        with_confirmation::propagate_deletion(TabsWithLoading::new(tabs, terms), original_term);
    } else {
        automatic::propagate_deletion(terms, tabs, original_term);
        terms.delete(&original_term.meta.term.name);
        tabs.term_tabs.close(&original_term.meta.term.name);
    }
}

pub(crate) use with_confirmation::commit::finish as finish_commit;

fn repeat_ongoing_commit_changes(
    term_tabs: &mut TermTabs<TermScreen>,
    commit_tabs: &mut TermTabs<Rc<RefCell<TwoPhaseCommit>>>,
    original_term: &FatTerm,
    updated_term: FatTerm,
) {
    let updated_term_name = updated_term.meta.term.name.clone();
    let previously_mentioned_terms = original_term.mentioned_terms();
    let currently_mentioned_terms = updated_term.mentioned_terms();

    let newly_mentioned_terms: HashSet<String> = currently_mentioned_terms
        .difference(&previously_mentioned_terms)
        .cloned()
        .collect();

    let mut need_to_transfer_to_commit = false;
    for c in commit_tabs.iter() {
        if newly_mentioned_terms.contains(&c.borrow().term.name()) {
            need_to_transfer_to_commit = true;
            break;
        }
    }
    if !need_to_transfer_to_commit {
        return;
    }

    if commit_tabs.get(&original_term.meta.term.name).is_none() {
        let tab = term_tabs
            .close(&updated_term_name)
            .expect("term was just updated");
        commit_tabs.push(&tab.extract_term());
    }

    let mut relevant_tabs = commit_tabs.borrow_mut(
        &newly_mentioned_terms
            .into_iter()
            .chain(std::iter::once(updated_term_name.clone()))
            .collect::<Vec<String>>(),
    );

    let mut updated_tab = relevant_tabs
        .iter()
        .position(|tab| tab.borrow().term.name() == updated_term_name)
        .map(|idx| relevant_tabs.swap_remove(idx))
        .expect("tab was just opened");

    for mentioned_tab in relevant_tabs {
        let (original_mentioned, mentioned_args_changes, updated_mentioned) =
            mentioned_tab.borrow().term.get_pits().accumulated_changes();

        if let Some(new_pit) = changes::propagation::apply(
            &original_mentioned,
            &mentioned_args_changes,
            &updated_mentioned,
            &updated_term,
        )
        .get(&updated_term_name)
        {
            updated_tab.borrow_mut().term.get_pits_mut().0.push_pit(
                &[],
                new_pit,
                &mentioned_tab.borrow().term.name(),
            );

            with_confirmation::add_approvers(mentioned_tab, &mut [&mut updated_tab]);
        }
    }
    let updated_tab_pits_count = updated_tab.borrow().term.get_pits().len();
    updated_tab
        .borrow_mut()
        .term
        .choose_pit(updated_tab_pits_count - 1);
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        model::{
            comment::{comment::Comment, name_description::NameDescription},
            term::{
                args_binding::ArgsBinding, bound_term::BoundTerm, rule::parse_rule, term::Term,
            },
        },
        term_knowledge_base::InMemoryTerms,
        ui::widgets::drag_and_drop,
    };

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
                    parse_rule("original(FirstRuleArg):-mentioned(FirstRuleArg)")
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
                    parse_rule("referring(FirstRuleArg):-original(FirstRuleArg)")
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

        let term_changes = vec![
            TermChange::RuleChanges,
            TermChange::FactsChange,
            TermChange::DescriptionChange,
        ];

        // only original tab is opened
        tabs.close("referring");
        tabs.close("mentioned");

        // should trigger automatic changes
        handle_changes(
            &mut tabs,
            &mut database,
            &original,
            &term_changes,
            updated.clone(),
        );

        assert!(tabs.get("original").is_none());
        assert!(database.get("original").is_none());

        assert_eq!(tabs.get("new_name").unwrap().extract_term(), updated);
        assert_eq!(database.get("new_name").unwrap(), updated);

        // should not open the related terms in tabs as this is automatic
        assert!(tabs.get("referring").is_none());
        assert!(tabs.get("mentioned").is_none());

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
        let term_changes = vec![TermChange::ArgChanges(vec![drag_and_drop::Change::Pushed(
            crate::model::comment::name_description::NameDescription::new("some", "arg"),
        )])];
        updated.meta.args.push(NameDescription::new("some", "arg"));
        updated.term.rules[0].head.binding.push("_".to_string());

        // only original tab is opened
        tabs.close("referring");
        tabs.close("mentioned");

        // should trigger automatic changes
        handle_changes(
            &mut tabs,
            &mut database,
            &original,
            &term_changes,
            updated.clone(),
        );

        assert_eq!(tabs.get("original").unwrap().extract_term(), updated);
        assert_eq!(database.get("original").unwrap(), updated);

        assert_eq!(database.get("referring").unwrap(), before_change_referring);
        assert_eq!(database.get("mentioned").unwrap(), before_change_mentioned);

        // should not open the related terms in tabs as this is automatic
        assert!(tabs.get("referring").is_none());
        assert!(tabs.get("mentioned").is_none());
    }

    #[test]
    fn when_args_changes_and_affected_and_new() {
        let (mut tabs, mut database) = setup();
        let mut original = database.get("original").unwrap();
        let mut updated = original.clone();
        // a newly created term can't be referred by any other terms
        updated.remove_referred_by("referring");

        // the arg is essentially a new arg
        let term_changes = vec![TermChange::ArgChanges(vec![drag_and_drop::Change::Pushed(
            updated.meta.args[0].clone(),
        )])];

        // simulate starting with a blank term
        original = FatTerm::default();
        database.delete("original");
        tabs.close("original");
        tabs.push(&original);
        let mut mentioned = database.get("mentioned").unwrap();
        mentioned.remove_referred_by("original");
        database.put("mentioned", mentioned).unwrap();

        let before_change_referring = database.get("referring").unwrap();

        // only original tab is opened
        tabs.close("referring");
        tabs.close("mentioned");

        // should trigger automatic changes
        handle_changes(
            &mut tabs,
            &mut database,
            &original,
            &term_changes,
            updated.clone(),
        );

        assert_eq!(tabs.get("original").unwrap().extract_term(), updated);
        assert_eq!(database.get("original").unwrap(), updated);

        assert_eq!(database.get("referring").unwrap(), before_change_referring);
        assert_eq!(
            database.get("mentioned").unwrap().meta.referred_by,
            vec!["original"]
        );

        // should not open the related terms in tabs as this is automatic
        assert!(tabs.get("referring").is_none());
        assert!(tabs.get("mentioned").is_none());
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
        let term_changes = vec![
            TermChange::ArgChanges(vec![drag_and_drop::Change::Pushed(NameDescription::new(
                "new_arg",
                "description",
            ))]),
            TermChange::RuleChanges,
        ];
        updated.meta.args[0] = NameDescription::new("new_arg", "description");
        updated.term.rules[0].head.binding.push("_".to_string());
        updated.term.rules[0].body.push(BoundTerm {
            name: "newly_mentioned".to_string(),
            arg_bindings: ArgsBinding {
                binding: vec!["woah".to_string()],
            },
        });

        // only original tab is opened
        tabs.close("referring");
        tabs.close("mentioned");

        // should trigger with_confirmation changes
        handle_changes(
            &mut tabs,
            &mut database,
            &original,
            &term_changes,
            updated.clone(),
        );

        assert_eq!(database.get("original").unwrap(), original);
        assert_eq!(database.get("referring").unwrap(), before_change_referring);
        assert_eq!(database.get("mentioned").unwrap(), before_change_mentioned);
        assert_eq!(database.get("newly_mentioned").unwrap(), newly_mentioned);

        // there is no actual change to mentioned so it doesn't need to be opened
        assert!(tabs.get("mentioned").is_none());

        {
            // referring needs to confirm the change so it must be opened
            let referring_tab = tabs.get("referring");
            assert!(referring_tab.is_some());
            let referring_tab = referring_tab.unwrap();

            // there's a newly mentioned term that will be changed due to this change
            let newly_mentioned_tab = tabs.get("newly_mentioned");
            assert!(newly_mentioned_tab.is_some());
            let newly_mentioned_tab = newly_mentioned_tab.unwrap();

            let original_tab = tabs.get("original").unwrap();

            // 2-phase-commit must be setup
            assert!(original_tab.two_phase_commit.is_some());
            assert!(referring_tab.two_phase_commit.is_some());
            assert!(newly_mentioned_tab.two_phase_commit.is_some());

            let original_two_phase_commit =
                original_tab.two_phase_commit.as_ref().unwrap().borrow();

            let mut original_awaiting_for: Vec<String> =
                original_two_phase_commit.waiting_for().cloned().collect();

            original_awaiting_for.sort();
            let mut expected_awaitng_for = vec!["referring", "newly_mentioned"];
            expected_awaitng_for.sort();
            assert_eq!(original_awaiting_for, expected_awaitng_for);
            assert_eq!(original_two_phase_commit.origin(), "original");
            assert!(original_two_phase_commit.is_initiator());
            assert!(!original_two_phase_commit.is_being_waited());

            let check_approvers = |approver_tab: &crate::ui::widgets::term_screen::TermScreen| {
                let approver_two_phase_commit =
                    approver_tab.two_phase_commit.as_ref().unwrap().borrow();

                let approver_awaiting_for: Vec<String> =
                    approver_two_phase_commit.waiting_for().cloned().collect();

                assert!(approver_awaiting_for.is_empty());
                assert_eq!(approver_two_phase_commit.origin(), "original");
                assert!(!approver_two_phase_commit.is_initiator());
                assert!(approver_two_phase_commit.is_being_waited());
            };

            check_approvers(referring_tab);
            check_approvers(newly_mentioned_tab);
        }

        // now test what happpens when the newly mentioned is changed while the 2-phase-commit is
        // in process
        let term_changes = vec![TermChange::ArgChanges(vec![drag_and_drop::Change::Pushed(
            NameDescription::new("new_arg", "description"),
        )])];
        let newly_mentioned = tabs.get("newly_mentioned").unwrap().extract_term();
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
            &term_changes,
            updated,
        );

        {
            // referring needs to confirm the change so it must be opened
            let referring_tab = tabs.get("referring");
            assert!(referring_tab.is_some());
            let referring_tab = referring_tab.unwrap();

            let newly_mentioned_tab = tabs.get("newly_mentioned");
            assert!(newly_mentioned_tab.is_some());
            let newly_mentioned_tab = newly_mentioned_tab.unwrap();

            let original_tab = tabs.get("original").unwrap();

            // 2-phase-commit must still be setup
            assert!(original_tab.two_phase_commit.is_some());
            assert!(referring_tab.two_phase_commit.is_some());
            assert!(newly_mentioned_tab.two_phase_commit.is_some());

            let original_two_phase_commit =
                original_tab.two_phase_commit.as_ref().unwrap().borrow();

            let mut original_awaiting_for: Vec<String> =
                original_two_phase_commit.waiting_for().cloned().collect();

            original_awaiting_for.sort();
            let mut expected_awaitng_for = vec!["referring", "newly_mentioned"];
            expected_awaitng_for.sort();
            assert_eq!(original_awaiting_for, expected_awaitng_for);
            assert_eq!(original_two_phase_commit.origin(), "original");
            assert!(original_two_phase_commit.is_initiator());
            // because of the change in newly_mentioned
            assert!(original_two_phase_commit.is_being_waited());

            let check_approvers = |approver_tab: &crate::ui::widgets::term_screen::TermScreen| {
                let approver_two_phase_commit =
                    approver_tab.two_phase_commit.as_ref().unwrap().borrow();

                assert_eq!(approver_two_phase_commit.origin(), "original");
                assert!(!approver_two_phase_commit.is_initiator());
                assert!(approver_two_phase_commit.is_being_waited());
            };

            check_approvers(referring_tab);
            check_approvers(newly_mentioned_tab);

            let reffering_two_phase_commit =
                referring_tab.two_phase_commit.as_ref().unwrap().borrow();

            let referring_waiting_for: Vec<String> =
                reffering_two_phase_commit.waiting_for().cloned().collect();
            assert!(referring_waiting_for.is_empty());

            let newly_mentioned_two_phase_commit = newly_mentioned_tab
                .two_phase_commit
                .as_ref()
                .unwrap()
                .borrow();

            let newly_mentioned_waiting_for: Vec<String> = newly_mentioned_two_phase_commit
                .waiting_for()
                .cloned()
                .collect();
            assert_eq!(newly_mentioned_waiting_for, &["original"]);
        }
    }
}
