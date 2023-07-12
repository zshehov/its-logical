use std::{cell::RefCell, rc::Rc};

use tracing::debug;

use crate::{
    changes::{self, ArgsChange},
    model::fat_term::FatTerm,
    term_knowledge_base::GetKnowledgeBase,
    ui::widgets::term_screen::{two_phase_commit::TwoPhaseCommit, TermScreen},
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
            affected_term.put(&updated_term.meta.term.name, &vec![], updated);
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
            affected_term.put(&term.meta.term.name, &vec![], updated);
        }
    }
}

pub(crate) fn add_approvers(
    source_two_phase_commit: &Rc<RefCell<TwoPhaseCommit>>,
    approvers: &mut [&mut TermScreen],
) {
    let origin_name = source_two_phase_commit.borrow().origin();

    let mut approvers_names = Vec::with_capacity(approvers.len());
    for approver in approvers {
        debug!("Adding approver {}", approver.name());
        approver
            .two_phase_commit
            .get_or_insert(Rc::new(RefCell::new(TwoPhaseCommit::new(
                &origin_name,
                false,
            ))))
            .borrow_mut()
            .add_approval_waiter(Rc::clone(source_two_phase_commit));
        approvers_names.push(approver.name());
    }
    source_two_phase_commit
        .borrow_mut()
        .append_approval_from(&approvers_names);
}

impl<H> GetKnowledgeBase for &[&mut H]
where
    H: loaded::TermHolder,
{
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        for term in self.iter() {
            let term = term.get();
            if term.meta.term.name == term_name {
                return Some(term.clone());
            }
        }
        None
    }
}
