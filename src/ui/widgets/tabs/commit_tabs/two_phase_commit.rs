use std::{cell::RefCell, rc::Rc};

use crate::{
    term_knowledge_base::GetKnowledgeBase,
    ui::widgets::term_screen::{Output, TermScreen},
};

pub(crate) struct TwoPhaseCommit {
    depending_on: Vec<(Rc<RefCell<TwoPhaseCommit>>, Rc<RefCell<bool>>)>,
    for_approval: Vec<Rc<RefCell<bool>>>,
    pub(crate) term: TermScreen,
}

impl TwoPhaseCommit {
    pub(crate) fn new(term_screen: TermScreen) -> Self {
        Self {
            depending_on: vec![],
            for_approval: vec![],
            term: term_screen,
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

    pub(crate) fn waiting_for(&self) -> impl Iterator<Item = String> + '_ {
        self.depending_on
            .iter()
            .filter(|(_, approved)| !*approved.borrow())
            .map(|(c, _)| c.borrow().term.name())
    }

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

impl TwoPhaseCommit {
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        terms_knowledge_base: &impl GetKnowledgeBase,
    ) -> Option<Output> {
        // if this term is a part of a 2-phase-commit and should approve a change show the approve
        // button
        if self.is_being_waited() && ui.button("approve").clicked() {
            self.approve_all();
        }
        self.term.show(ui, terms_knowledge_base)
    }
}
