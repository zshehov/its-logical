use std::{cell::RefCell, collections::HashSet, rc::Rc};

pub(crate) struct TwoPhaseCommit {
    origin: String,
    is_initiator: bool,
    waiting_for_approval_from: HashSet<String>,
    had_approval_from: HashSet<String>,
    awaiting_approval: Vec<Rc<RefCell<TwoPhaseCommit>>>,
}

impl TwoPhaseCommit {
    pub(crate) fn new(origin: &str, is_initiator: bool) -> Self {
        Self {
            waiting_for_approval_from: HashSet::new(),
            had_approval_from: HashSet::new(),
            awaiting_approval: vec![],
            is_initiator,
            origin: origin.to_string(),
        }
    }

    pub(crate) fn is_initiator(&self) -> bool {
        self.is_initiator
    }

    pub(crate) fn is_being_waited(&self) -> bool {
        !self.awaiting_approval.is_empty()
    }

    pub(crate) fn approve_all(&mut self, name: &str) {
        for change_source_term in &mut self.awaiting_approval {
            change_source_term.borrow_mut().approve_from(name);
        }
        self.awaiting_approval.clear();
    }

    pub(crate) fn add_approval_waiter(&mut self, waiter: Rc<RefCell<TwoPhaseCommit>>) {
        self.awaiting_approval.push(waiter);
    }

    pub(crate) fn append_approval_from(&mut self, approval_from: &[String]) {
        self.waiting_for_approval_from
            .extend(approval_from.iter().cloned());
    }

    pub(crate) fn iter_approved(&self) -> impl Iterator<Item = String> + '_ {
        self.had_approval_from.iter().cloned()
    }

    pub(crate) fn waiting_for(&self) -> impl ExactSizeIterator<Item = &String> {
        self.waiting_for_approval_from.iter()
    }

    pub(crate) fn origin(&self) -> String {
        self.origin.clone()
    }

    fn approve_from(&mut self, approved: &str) {
        if self.waiting_for_approval_from.remove(approved) {
            self.had_approval_from.insert(approved.to_owned());
        }
    }
}
