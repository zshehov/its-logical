use std::{cell::RefCell, rc::Rc};

use crate::{
    model::fat_term::FatTerm,
    ui::widgets::term_screen::{two_phase_commit::TwoPhaseCommit, TermScreen},
};

pub(crate) trait Screen {
    fn new(term: &FatTerm) -> Self;
    fn can_close(&self) -> bool;
    fn name(&self) -> String;
}

impl Screen for TermScreen {
    fn new(term: &FatTerm) -> Self {
        TermScreen::new(term, false)
    }

    fn can_close(&self) -> bool {
        !self.in_edit()
    }

    fn name(&self) -> String {
        self.name()
    }
}

impl Screen for Rc<RefCell<TwoPhaseCommit>> {
    fn new(term: &FatTerm) -> Self {
        Rc::new(RefCell::new(TwoPhaseCommit::new(TermScreen::new(
            term, false,
        ))))
    }

    fn can_close(&self) -> bool {
        // screen a part of a two phase commits shouldn't be closed
        false
    }

    fn name(&self) -> String {
        self.borrow().term.name()
    }
}
