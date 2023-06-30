use std::{cell::RefCell, collections::HashMap, rc::Rc};

use tracing::debug;

use crate::{
    model::fat_term::FatTerm,
    term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::{tabs::Tabs, term_screen::two_phase_commit::TwoPhaseCommit},
};

use super::OpenedTermScreens;

pub(crate) mod change;
pub(crate) mod deletion;

fn setup_with_confirmation<'a>(
    tabs: &'a mut Tabs,
    terms: &impl TermsKnowledgeBase,
    original_term: &FatTerm,
    affected: &[String],
) -> OpenedTermScreens<'a> {
    debug!("Need confirmation");

    let two_phase_commit = Rc::clone(
        tabs.get_mut(&original_term.meta.term.name)
            .expect("a change is coming from an opened term screen")
            .two_phase_commit
            .get_or_insert(Rc::new(RefCell::new(TwoPhaseCommit::new(
                &original_term.meta.term.name,
                true,
            )))),
    );
    two_phase_commit.borrow_mut().append_approval_from(affected);

    // TODO: Match instead of unwrap here
    // On failure - remove the two_phase_commit initiated above
    let mut opened_affected_term_screens = validate_two_phase(
        tabs,
        terms,
        &two_phase_commit.borrow(),
        &original_term.meta.term.name,
        affected,
    )
    .unwrap();
    adjust_two_phase_commits(&two_phase_commit, &mut opened_affected_term_screens);
    opened_affected_term_screens
}

fn adjust_two_phase_commits(
    two_phase_commit: &Rc<RefCell<TwoPhaseCommit>>,
    loaded_related_terms: &mut OpenedTermScreens<'_>,
) {
    let origin_name = two_phase_commit.borrow().origin();

    for opened_affected in &mut loaded_related_terms.affected {
        debug!("Adding affected {}", opened_affected.name());
        opened_affected
            .two_phase_commit
            .get_or_insert(Rc::new(RefCell::new(TwoPhaseCommit::new(
                &origin_name,
                false,
            ))))
            .borrow_mut()
            .add_approval_waiter(Rc::clone(&two_phase_commit));
    }
}

fn validate_two_phase<'a>(
    tabs: &'a mut Tabs,
    terms: &impl TermsKnowledgeBase,
    two_phase_commit: &TwoPhaseCommit,
    initiator: &str,
    affected: &[String],
) -> Option<OpenedTermScreens<'a>> {
    if affected
        .iter()
        .any(|affected_name| match tabs.get(affected_name) {
            Some(affected_term_screen) => {
                !affected_term_screen.is_ready_for_change(&two_phase_commit.origin())
            }
            None => false,
        })
    {
        return None;
    }

    for affected_term_name in affected {
        if tabs.get(&affected_term_name).is_none() {
            tabs.push(&terms.get(&affected_term_name).unwrap());
        }
    }

    let mut with_initiator: Vec<String> = Vec::with_capacity(affected.len() + 1);
    with_initiator.extend_from_slice(affected);
    with_initiator.push(initiator.to_owned());

    let mut all_term_screens = tabs.borrow_mut(&with_initiator);
    let initiator = all_term_screens.swap_remove(
        all_term_screens
            .iter()
            .position(|x| x.name() == initiator)
            .unwrap(),
    );

    Some(OpenedTermScreens {
        affected: all_term_screens,
        initiator,
    })
}

/// push_updated_pits goes through the `loaded_term_screens` and pushes the updated Point in time
/// if it's present in the `updates` HashMap
fn push_updated_pits(
    updates: HashMap<String, FatTerm>,
    update_source: &str,
    loaded_term_screens: &mut OpenedTermScreens<'_>,
) {
    for loaded in loaded_term_screens
        .affected
        .iter_mut()
        .chain(std::iter::once(&mut loaded_term_screens.initiator))
    {
        if let Some(updated) = updates.get(&loaded.name()) {
            loaded.get_pits_mut().0.push_pit(&updated, update_source);
            loaded.choose_pit(loaded.get_pits().len() - 1);
        }
    }
}
