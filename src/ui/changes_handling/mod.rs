use tracing::debug;

use crate::{
    changes,
    model::{comment::name_description::NameDescription, fat_term::FatTerm},
    term_knowledge_base::TermsKnowledgeBase,
};

use super::widgets::{
    drag_and_drop::Change,
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
    mut updated_term: FatTerm,
) {
    // only argument changes are tough and need special care
    let arg_changes = term_changes
        .iter()
        .find_map(|change| {
            if let TermChange::ArgChanges(arg_changes) = change {
                return Some(convert_args_changes(&arg_changes));
            }
            None
        })
        .unwrap_or(vec![]);

    // the user doesn't actually finish the updating due to argument
    // changes, so this is done here
    // TODO: this is not idempotent operation
    changes::propagation::finish_self_term(&mut updated_term, &arg_changes);

    let affected =
        changes::propagation::affected_from_changes(&original_term, &updated_term, &arg_changes);

    if original_term.meta.term.name == "".to_string() {
        // That's a new term - directly apply
        return automatic::change::propagate(
            tabs,
            terms,
            &original_term,
            &arg_changes,
            &updated_term,
            &affected,
        );
    }

    debug!(
        "Changes made for {}. Propagating to: {:?}",
        original_term.meta.term.name, affected
    );

    if arg_changes.is_empty() || affected.len() == 0 {
        automatic::change::propagate(
            tabs,
            terms,
            &original_term,
            &arg_changes,
            &updated_term,
            &affected,
        );
    } else {
        with_confirmation::change::propagate(
            tabs,
            terms,
            original_term,
            &arg_changes,
            &updated_term,
            &affected,
        );
    }
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

fn convert_args_changes(input: &[Change<NameDescription>]) -> Vec<changes::ArgsChange> {
    input
        .iter()
        .map(|change| match change {
            Change::Pushed(arg) => changes::ArgsChange::Pushed(arg.name.clone()),
            Change::Moved(moves) => changes::ArgsChange::Moved(moves.to_owned()),
            Change::Removed(idx, arg) => changes::ArgsChange::Removed(*idx, arg.name.clone()),
        })
        .collect()
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
