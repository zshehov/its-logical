use std::collections::HashSet;

use tracing::debug;

use crate::{
    changes,
    model::fat_term::FatTerm,
    term_knowledge_base::{DeleteKnowledgeBase, GetKnowledgeBase, PutKnowledgeBase},
};

use self::with_confirmation::loaded::TabsWithLoading;

use super::widgets::{tabs::Tabs, term_screen::term_screen_pit::TermChange};

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
    repeat_ongoing_commit_changes(tabs, original_term, updated_term);
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
        tabs.close(&original_term.meta.term.name);
    }
}

pub(crate) use with_confirmation::commit::finish as finish_commit;

fn repeat_ongoing_commit_changes(
    tabs: &mut Tabs,
    original_term: &FatTerm,
    mut updated_term: FatTerm,
) {
    let updated_term_name = updated_term.meta.term.name.clone();
    let previously_mentioned_terms = original_term.mentioned_terms();
    let currently_mentioned_terms = updated_term.mentioned_terms();

    let newly_mentioned_terms = currently_mentioned_terms.difference(&previously_mentioned_terms);

    let mut relevant_tabs = tabs.borrow_mut(
        &newly_mentioned_terms
            .into_iter()
            .cloned()
            .chain(std::iter::once(updated_term_name.clone()))
            .collect::<Vec<String>>(),
    );

    let updated_tab = relevant_tabs.swap_remove(
        relevant_tabs
            .iter()
            .position(|tab| tab.name() == updated_term_name)
            .expect("just updated term must have a tab"),
    );

    for mentioned_tab in relevant_tabs {
        if let Some(commit) = &mentioned_tab.two_phase_commit {
            let (original_mentioned, mentioned_args_changes, updated_mentioned) =
                mentioned_tab.get_pits().accumulated_changes();

            updated_term = changes::propagation::apply(
                &original_mentioned,
                &mentioned_args_changes,
                &updated_mentioned,
                &updated_term,
            )
            .get(&updated_term_name)
            .unwrap()
            .clone();

            updated_tab
                .get_pits_mut()
                .0
                .push_pit(&[], &updated_term, &mentioned_tab.name());

            // relies on the invariant that there is only ever a single 2phase commit at a time
            with_confirmation::add_approvers(commit, &mut [updated_tab]);
        }
    }
    updated_tab.choose_pit(updated_tab.get_pits().len() - 1);
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        model::{
            comment::{comment::Comment, name_description::NameDescription},
            term::{rule::parse_rule, term::Term},
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
    /*
    #[test]
    fn when_args_changes_and_affected_and_not_new() {
        let mut original_term = FatTerm::default();
        original_term.meta.term.name = "not_new_term".to_string();
        original_term.meta.referred_by = vec!["some_other_term".to_string()];
        let updated_term = original_term.clone();

        // with arg change
        let term_changes = vec![TermChange::ArgChanges(vec![drag_and_drop::Change::Pushed(
            crate::model::comment::name_description::NameDescription::new("some", "arg"),
        )])];

        let (auto, with_confirmation) =
            setup_handle_term_screen_changes_internal(&original_term, &term_changes, updated_term);

        assert!(!*auto.propagate_called.borrow());
        assert!(*with_confirmation.propagate_called.borrow());
    }
    */
}
