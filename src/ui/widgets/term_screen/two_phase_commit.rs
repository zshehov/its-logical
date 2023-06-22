use std::{cell::RefCell, collections::HashSet, rc::Rc};

pub(crate) struct TwoPhaseCommit {
    waiting_for_approval_from: HashSet<String>,
    had_approval_from: HashSet<String>,
    awaiting_approval: Vec<Rc<RefCell<TwoPhaseCommit>>>,
}

impl TwoPhaseCommit {
    pub(crate) fn new() -> Self {
        Self {
            waiting_for_approval_from: HashSet::new(),
            had_approval_from: HashSet::new(),
            awaiting_approval: vec![],
        }
    }
    pub(crate) fn is_being_waited(&self) -> bool {
        self.awaiting_approval.len() > 0
    }

    pub(crate) fn approve_all(&mut self, name: &str) {
        for change_source_term in &mut self.awaiting_approval {
            change_source_term.borrow_mut().approve_from(name);
        }
        self.awaiting_approval.clear();
    }

    pub(crate) fn add_waiter_for_approval(&mut self, waiter: Rc<RefCell<TwoPhaseCommit>>) {
        self.awaiting_approval.push(waiter);
    }

    pub(crate) fn approve_from(&mut self, approved: &str) {
        if self.waiting_for_approval_from.remove(approved) {
            self.had_approval_from.insert(approved.to_owned());
        }
    }

    pub(crate) fn append_approval_from(&mut self, approval_from: &Vec<String>) {
        self.waiting_for_approval_from
            .extend(approval_from.iter().cloned());
    }

    pub(crate) fn iter_approved(&self) -> impl Iterator<Item = String> + '_ {
        self.had_approval_from.iter().cloned()
    }

    pub(crate) fn waits_for_approval(&self) -> bool {
        self.waiting_for_approval_from.len() > 0
    }
}
