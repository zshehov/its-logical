use egui::{Color32, Stroke};

use crate::{
    terms_cache::TwoPhaseTerm,
    ui::{tabs::two_phase_commit_screen::TwoPhaseCommitScreen, term_screen::TermScreen},
};

pub(crate) trait Screen {
    fn can_close(&self) -> bool;
    fn stroke(&self) -> Stroke;
}

impl Screen for TermScreen {
    fn can_close(&self) -> bool {
        !self.in_edit()
    }

    fn stroke(&self) -> Stroke {
        Stroke::NONE
    }
}

impl Screen for TwoPhaseCommitScreen {
    fn can_close(&self) -> bool {
        false
    }

    fn stroke(&self) -> Stroke {
        let two_phase = self.two_phase_commit().borrow();
        if two_phase.is_being_waited() {
            Stroke::new(3.0, Color32::RED)
        } else if two_phase.is_waiting() {
            Stroke::new(3.0, Color32::DARK_RED)
        } else {
            Stroke::new(3.0, Color32::GREEN)
        }
    }
}
