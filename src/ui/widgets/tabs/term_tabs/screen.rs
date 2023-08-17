use crate::knowledge::model::fat_term::FatTerm;
use std::{cell::RefCell, rc::Rc};

use egui::{Color32, Stroke};

use crate::ui::widgets::{
    tabs::commit_tabs::two_phase_commit::TwoPhaseCommit, term_screen::TermScreen,
};

pub(crate) trait Screen {
    fn new(term: &FatTerm) -> Self;
    fn can_close(&self) -> bool;
    fn name(&self) -> String;
    fn stroke(&self) -> Stroke;
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

    fn stroke(&self) -> Stroke {
        Stroke::NONE
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

    fn stroke(&self) -> Stroke {
        let screen = self.borrow();
        if screen.is_being_waited() {
            Stroke::new(3.0, Color32::RED)
        } else if screen.is_waiting() {
            Stroke::new(3.0, Color32::DARK_RED)
        } else {
            Stroke::new(3.0, Color32::GREEN)
        }
    }
}
