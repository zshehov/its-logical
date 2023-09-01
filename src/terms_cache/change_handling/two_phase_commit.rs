use std::{cell::RefCell, rc::Rc};

pub(crate) struct TwoPhaseCommit {
    depending_on: Vec<(Rc<RefCell<TwoPhaseCommit>>, Rc<RefCell<bool>>)>,
    for_approval: Vec<Rc<RefCell<bool>>>,
}

impl TwoPhaseCommit {
    pub(crate) fn new() -> Self {
        Self {
            depending_on: vec![],
            for_approval: vec![],
        }
    }

    pub(crate) fn is_being_waited(&self) -> bool {
        !self.for_approval.is_empty()
    }

    pub(crate) fn is_waiting(&self) -> bool {
        self.depending_on
            .iter()
            .any(|(_, approved)| !*approved.borrow())
    }

    // pub(crate) fn waiting_for(&self) -> impl Iterator<Item = String> + '_ {
    //     self.depending_on
    //         .iter()
    //         .filter(|(_, approved)| !*approved.borrow())
    //         .map(|(c, _)| c.borrow().term.name())
    // }

    pub(crate) fn approve_all(&mut self) {
        for r in &mut self.for_approval {
            *r.borrow_mut() = true;
        }
        self.for_approval.clear();
    }

    pub(crate) fn add_approval_waiter(&mut self, waiter: &Rc<RefCell<bool>>) {
        self.for_approval.push(Rc::clone(waiter));
    }

    pub(crate) fn wait_approval_from(
        &mut self,
        approval_from: &(Rc<RefCell<TwoPhaseCommit>>, Rc<RefCell<bool>>),
    ) {
        self.depending_on.push(approval_from.clone());
    }
}
