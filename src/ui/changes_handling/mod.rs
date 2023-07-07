use tracing::debug;

use crate::{
    changes,
    model::fat_term::FatTerm,
    term_knowledge_base::{GetKnowledgeBase, PutKnowledgeBase, TermsKnowledgeBase},
};

use self::with_confirmation::TabsWithLoading;

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

    let affected =
        changes::propagation::affected_from_changes(original_term, &updated_term, &arg_changes);

    debug!(
        "Changes made for {}. Propagating to: {:?}",
        original_name, affected
    );

    if
    /* the changes are not worthy of user confirmation */
    arg_changes.is_empty()
        || /* no other term is affected */ affected.is_empty()
        || /* a new term */ original_term.meta.term.name == *""
    {
        debug!("automatic propagation");
        automatic::propagate(
            terms,
            tabs,
            original_term,
            &arg_changes,
            &updated_term,
            &affected,
        );
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
    terms: &mut impl TermsKnowledgeBase,
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
                &automatic::SingleTerm::new(updated_term),
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
    use crate::ui::widgets::drag_and_drop;

    use super::*;
    use std::cell::RefCell;

    struct MockAutomaticPropagator {
        propagate_called: RefCell<bool>,
    }
    impl MockAutomaticPropagator {
        fn new() -> Self {
            Self {
                propagate_called: RefCell::new(false),
            }
        }
    }
    struct MockConfirmationPropagator {
        propagate_called: RefCell<bool>,
    }
    impl MockConfirmationPropagator {
        fn new() -> Self {
            Self {
                propagate_called: RefCell::new(false),
            }
        }
    }

    impl Propagator for &MockAutomaticPropagator {
        fn propagate(
            &self,
            _tabs: &mut Tabs,
            _terms: &mut impl TermsKnowledgeBase,
            _original_term: &FatTerm,
            _arg_changes: &[ArgsChange],
            _updated_term: &FatTerm,
            _affected: &[String],
        ) {
            *self.propagate_called.borrow_mut() = true;
        }
    }
    impl WithConfirmationPropagator for &MockConfirmationPropagator {
        fn propagate(
            &self,
            _tabs: &mut Tabs,
            _terms: &impl TermsKnowledgeBase,
            _original_term: &FatTerm,
            _arg_changes: &[ArgsChange],
            _updated_term: &FatTerm,
            _affected: &[String],
        ) {
            *self.propagate_called.borrow_mut() = true;
        }
    }

    struct MockTermsKnowledge {
        emtpy_vec: Vec<String>,
    }
    impl MockTermsKnowledge {
        fn new() -> Self {
            Self { emtpy_vec: vec![] }
        }
    }
    impl TermsKnowledgeBase for MockTermsKnowledge {
        fn get(&self, _: &str) -> Option<FatTerm> {
            None
        }

        fn put(
            &mut self,
            _: &str,
            _: FatTerm,
        ) -> Result<(), crate::term_knowledge_base::KnowledgeBaseError> {
            Ok(())
        }

        fn keys(&self) -> &Vec<String> {
            &self.emtpy_vec
        }

        fn delete(&mut self, _: &str) {}
    }

    /* handle_term_screen_changes_internal */

    fn setup_handle_term_screen_changes_internal(
        original_term: &FatTerm,
        term_changes: &[TermChange],
        updated_term: FatTerm,
    ) -> (MockAutomaticPropagator, MockConfirmationPropagator) {
        let mut tabs = Tabs::default();
        let mut terms = MockTermsKnowledge::new();

        let automatic_propagator = MockAutomaticPropagator::new();
        let with_confirmation_propagator = MockConfirmationPropagator::new();

        handle_term_screen_changes_internal(
            original_term,
            term_changes,
            updated_term,
            &automatic_propagator,
            &with_confirmation_propagator,
        );
        (automatic_propagator, with_confirmation_propagator)
    }

    #[test]
    fn when_empty_args_changes_with_affected_not_new() {
        let mut original_term = FatTerm::default();
        original_term.meta.referred_by = vec!["some_other_term".to_string()];
        original_term.meta.term.name = "bla".to_string();
        let updated_term = original_term.clone();

        let term_changes = vec![
            TermChange::RuleChanges,
            TermChange::FactsChange,
            TermChange::DescriptionChange,
        ];

        let (auto, with_confirmation) =
            setup_handle_term_screen_changes_internal(&original_term, &term_changes, updated_term);

        assert!(*auto.propagate_called.borrow());
        assert!(!*with_confirmation.propagate_called.borrow());
    }
    #[test]
    fn when_args_changes_and_no_affected_not_new() {
        let mut original_term = FatTerm::default();
        original_term.meta.term.name = "bla".to_string();
        let updated_term = original_term.clone();

        // with arg change
        let term_changes = vec![TermChange::ArgChanges(vec![drag_and_drop::Change::Pushed(
            crate::model::comment::name_description::NameDescription::new("some", "arg"),
        )])];

        let (auto, with_confirmation) =
            setup_handle_term_screen_changes_internal(&original_term, &term_changes, updated_term);

        assert!(*auto.propagate_called.borrow());
        assert!(!*with_confirmation.propagate_called.borrow());
    }
    #[test]
    fn when_args_changes_and_affected_and_new() {
        let original_term = FatTerm::default();
        let mut updated_term = original_term.clone();
        updated_term.meta.term.name = "new_term".to_string();
        updated_term.meta.referred_by = vec!["some_other_term".to_string()];

        // with arg change
        let term_changes = vec![TermChange::ArgChanges(vec![drag_and_drop::Change::Pushed(
            crate::model::comment::name_description::NameDescription::new("some", "arg"),
        )])];

        let (auto, with_confirmation) =
            setup_handle_term_screen_changes_internal(&original_term, &term_changes, updated_term);

        assert!(*auto.propagate_called.borrow());
        assert!(!*with_confirmation.propagate_called.borrow());
    }

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
}
