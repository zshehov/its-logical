use std::{cell::RefCell, rc::Rc};

use crate::ui::widgets::term_screen::two_phase_commit::TwoPhaseCommit;

use super::term_tabs::TermTabs;

pub(crate) enum CommitTabsOutput<'a> {
    Selected(&'a mut Rc<RefCell<TwoPhaseCommit>>),
    FinishedCommit,
}

pub(crate) struct CommitTabs {
    pub(crate) tabs: TermTabs<Rc<RefCell<TwoPhaseCommit>>>,
}

impl CommitTabs {
    pub(crate) fn new() -> Self {
        Self {
            tabs: TermTabs::new(),
        }
    }

    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> Option<CommitTabsOutput<'_>> {
        let commit_button = egui::Button::new("Finish commit");
        let ready_for_commit = self.tabs.iter().all(|x| !x.borrow().is_waiting());
        let mut output = None;
        if ui
            .add_enabled(ready_for_commit, commit_button)
            .on_disabled_hover_text("Still need some approvals")
            .clicked()
        {
            output = Some(CommitTabsOutput::FinishedCommit);
        }

        output.or(self.tabs.show(ui).map(|x| CommitTabsOutput::Selected(x)))
    }
}
