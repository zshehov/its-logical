use crate::knowledge::store::Get;
use std::{cell::RefCell, rc::Rc};

use tracing::debug;

use crate::{
    changes::{self, ArgsChange},
    model::fat_term::FatTerm,
    ui::widgets::tabs::commit_tabs::two_phase_commit::TwoPhaseCommit,
};

pub(crate) mod commit;
pub(crate) mod loaded;
use loaded::TermHolder;

pub(crate) fn propagate(
    mut loaded: impl loaded::Loaded,
    original_term: &FatTerm,
    arg_changes: &[ArgsChange],
    updated_term: &FatTerm,
    affected: &[String],
) {
    let (initiator, affected) = loaded
        .borrow_mut(&original_term.meta.term.name, affected)
        .expect("[TODO] inability to load is not handled");

    let updates =
        changes::propagation::apply(original_term, arg_changes, updated_term, &affected.as_ref());

    initiator.put(&updated_term.meta.term.name, arg_changes, updated_term);
    for affected_term in affected {
        if let Some(updated) = updates.get(&affected_term.get().meta.term.name) {
            affected_term.put(&updated_term.meta.term.name, &[], updated);
        }
    }
}

pub(crate) fn propagate_deletion(mut loaded: impl loaded::Loaded, term: &FatTerm) {
    let (_, affected) = loaded
        .borrow_mut(
            &term.meta.term.name,
            &changes::propagation::affected_from_deletion(term),
        )
        .expect("[TODO] inability to load is not handled");

    let updates = changes::propagation::apply_deletion(term, &affected.as_ref());

    for affected_term in affected {
        if let Some(updated) = updates.get(&affected_term.get().meta.term.name) {
            affected_term.put(&term.meta.term.name, &[], updated);
        }
    }
}

pub(crate) fn add_approvers(
    source_two_phase_commit: &Rc<RefCell<TwoPhaseCommit>>,
    approvers: &mut [&mut Rc<RefCell<TwoPhaseCommit>>],
) {
    for approver in approvers {
        debug!("Adding approver {}", approver.borrow().term.name());

        let approve = Rc::new(RefCell::new(false));

        approver.borrow_mut().add_approval_waiter(&approve);

        source_two_phase_commit
            .borrow_mut()
            .wait_approval_from(&(approver.to_owned(), approve));
    }
}

impl<H> Get for &[&mut H]
where
    H: loaded::TermHolder,
{
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        for term in self.iter() {
            let term = term.get();
            if term.meta.term.name == term_name {
                return Some(term);
            }
        }
        None
    }
}
