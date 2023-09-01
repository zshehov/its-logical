use std::{cell::RefCell, rc::Rc};

use its_logical::knowledge::{
    model::fat_term::FatTerm,
    store::{Get, Keys},
};

use crate::{
    terms_cache::change_handling::two_phase_commit::TwoPhaseCommit,
    ui::term_screen::{
        points_in_time::PointsInTime, term_screen_pit::TermScreenPIT, Output, TermScreen,
    },
};

pub(crate) struct TwoPhaseCommitScreen {
    commit: Rc<RefCell<TwoPhaseCommit>>,
    screen: TermScreen,
}

impl crate::terms_cache::NamedTerm for TwoPhaseCommitScreen {
    fn new(term: FatTerm) -> Self {
        Self::new(TermScreen::new(&term, false))
    }

    fn name(&self) -> String {
        self.name()
    }

    fn term(&self) -> FatTerm {
        self.extract_term()
    }
}

impl crate::terms_cache::TwoPhaseTerm for TwoPhaseCommitScreen {
    type Creator = TermScreen;
    fn from(creator: Self::Creator) -> Self {
        Self {
            commit: Rc::new(RefCell::new(TwoPhaseCommit::new())),
            screen: creator,
        }
    }

    fn two_phase_commit(&self) -> &Rc<RefCell<TwoPhaseCommit>> {
        &self.commit
    }

    fn current_change(&self) -> its_logical::changes::change::Change {
        let (original, args_changes, changed) = self.screen.get_pits().accumulated_changes();

        its_logical::changes::change::Change::new(original, &args_changes, changed)
    }
}

impl crate::terms_cache::change_handling::ConfirmationApply for TwoPhaseCommitScreen {
    fn push_for_confirmation(
        &mut self,
        arg_changes: &[its_logical::changes::change::ArgsChange],
        resulting_term: &FatTerm,
        source: &str,
    ) {
        let pits = self.get_pits_mut().0;

        pits.push_pit(arg_changes, resulting_term, source);
        let pits_count = pits.len();
        self.choose_pit(pits_count - 1);
    }
}

impl crate::terms_cache::change_handling::AutoApply for TwoPhaseCommitScreen {
    fn apply(&mut self, f: impl Fn(&FatTerm) -> FatTerm) {
        self.screen.apply(f);
    }
}

impl TwoPhaseCommitScreen {
    pub(crate) fn new(s: TermScreen) -> Self {
        Self {
            commit: Rc::new(RefCell::new(TwoPhaseCommit::new())),
            screen: s,
        }
    }

    pub(crate) fn name(&self) -> String {
        self.screen.name()
    }

    pub(crate) fn extract_term(&self) -> FatTerm {
        self.screen.extract_term()
    }

    pub(crate) fn get_pits_mut(&mut self) -> (&mut PointsInTime, Option<&mut TermScreenPIT>) {
        self.screen.get_pits_mut()
    }

    pub(crate) fn choose_pit(&mut self, pit_idx: usize) {
        self.screen.choose_pit(pit_idx);
    }
}

impl TwoPhaseCommitScreen {
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        terms_knowledge_base: &(impl Get + Keys),
    ) -> Option<Output> {
        // if this term is a part of a 2-phase-commit and should approve a change show the approve
        // button
        if self.commit.borrow().is_being_waited() && ui.button("approve").clicked() {
            self.commit.borrow_mut().approve_all();
        }
        self.screen.show(ui, terms_knowledge_base)
    }
}
