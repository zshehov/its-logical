use its_logical::{
    changes::{
        change::{self, Apply},
        deletion::Deletion,
    },
    knowledge::model::fat_term::FatTerm,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use tracing::debug;

pub(crate) mod commit;
pub(crate) mod loaded;
use loaded::TermHolder;

use crate::ui::tabs::commit_tabs::two_phase_commit::TwoPhaseCommit;

pub(crate) fn propagate(
    mut loaded: impl loaded::Loaded,
    original_term: &FatTerm,
    arg_changes: &[change::ArgsChange],
    updated_term: &FatTerm,
    affected: &[String],
) {
    let (initiator, affected) = loaded
        .borrow_mut(&original_term.meta.term.name, affected)
        .expect("[TODO] inability to load is not handled");

    let change = change::Change::new(
        original_term.to_owned(),
        arg_changes,
        updated_term.to_owned(),
    );

    let mut updates = HashMap::with_capacity(affected.len());

    for loaded_term in affected.iter() {
        let term = loaded_term.get();
        let changed = term.apply(&change);
        updates.extend(changed);
    }

    initiator.put(&updated_term.meta.term.name, arg_changes, updated_term);
    for affected_term in affected {
        if let Some(updated) = updates.get(&affected_term.get().meta.term.name) {
            affected_term.put(&updated_term.meta.term.name, &[], updated);
        }
    }
}

pub(crate) fn propagate_deletion(mut loaded: impl loaded::Loaded, deleted_term: &FatTerm) {
    let (_, affected) = loaded
        .borrow_mut(
            &deleted_term.meta.term.name,
            &deleted_term.deletion_affects(),
        )
        .expect("[TODO] inability to load is not handled");

    let mut updates = HashMap::with_capacity(affected.len());

    for loaded_term in affected.iter() {
        let term = loaded_term.get();
        let changed = term.apply_deletion(deleted_term);
        updates.extend(changed);
    }

    for affected_term in affected {
        if let Some(updated) = updates.get(&affected_term.get().meta.term.name) {
            affected_term.put(&deleted_term.meta.term.name, &[], updated);
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
