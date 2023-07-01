use tracing::debug;

use crate::{changes, model::fat_term::FatTerm, term_knowledge_base::TermsKnowledgeBase};

use super::widgets::{
    tabs::Tabs,
    term_screen::{term_screen_pit::TermChange, TermScreen},
};

mod automatic;
mod with_confirmation;

pub(crate) fn handle_term_screen_changes(
    tabs: &mut Tabs,
    terms: &mut impl TermsKnowledgeBase,
    original_term: &FatTerm,
    term_changes: &[TermChange],
    updated_term: FatTerm,
) {
    let original_name = original_term.meta.term.name.clone();
    // only argument changes are tough and need special care
    let arg_changes = term_changes
        .into_iter()
        .find_map(|change| {
            if let TermChange::ArgChanges(arg_changes) = change {
                return Some(arg_changes.into_iter().map(|x| x.into()).collect());
            }
            None
        })
        .unwrap_or(vec![]);

    let affected =
        changes::propagation::affected_from_changes(&original_term, &updated_term, &arg_changes);

    debug!(
        "Changes made for {}. Propagating to: {:?}",
        original_name, affected
    );

    if
    /* the changes are not worthy of user confirmation */
    arg_changes.is_empty()
        || /* no other term is affected */ affected.len() == 0
        || /* a new term */ original_term.meta.term.name == "".to_string()
    {
        debug!("automatic propagation");
        automatic::change::propagate(
            tabs,
            terms,
            &original_term,
            &arg_changes,
            &updated_term,
            &affected,
        );
    } else {
        debug!("2 phase commit propagation");
        with_confirmation::change::propagate(
            tabs,
            terms,
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
        with_confirmation::deletion::propagate(tabs, terms, original_term);
    } else {
        automatic::deletion::propagate(
            tabs,
            terms,
            original_term,
            &changes::propagation::affected_from_deletion(original_term),
        );
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
                .push_pit(&vec![], &updated_term, &mentioned_tab.name());

            // relies on the invariant that there is only ever a single 2phase commit at a time
            with_confirmation::add_approvers(commit, &mut [updated_tab]);
        }
    }
}

pub(crate) struct OpenedTermScreens<'a> {
    initiator: &'a mut TermScreen,
    affected: Vec<&'a mut TermScreen>,
}

impl<'a> changes::propagation::Terms for OpenedTermScreens<'a> {
    fn get(&self, term_name: &str) -> Option<crate::model::fat_term::FatTerm> {
        if term_name == self.initiator.name() {
            return Some(self.initiator.extract_term());
        }
        for screen in &self.affected {
            if screen.name() == term_name {
                return Some(screen.extract_term());
            }
        }
        None
    }
}
