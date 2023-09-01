use std::cmp::min;

use screen::Screen;

use crate::{
    terms_cache::{TermsCache, TwoPhaseTerm},
    ui::term_screen::TermScreen,
};

use super::{two_phase_commit_screen::TwoPhaseCommitScreen, ChosenTab};

pub(crate) mod screen;

pub(crate) enum Output {
    FinishedCommit,
    AbortedCommit,
}

impl TermsCache<TermScreen, TwoPhaseCommitScreen> {
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        current_tab: &mut ChosenTab,
    ) -> Option<Output> {
        let mut output = None;
        let mut has_two_phase_commit = false;
        let mut is_ready_for_commit = true;

        ui.horizontal(|ui| {
            let mut close_idx = None;
            for (idx, screen) in self.iter_mut().enumerate() {
                let (name, stroke, can_close) = match screen {
                    crate::terms_cache::TermHolder::Normal(s) => {
                        (s.name(), s.stroke(), s.can_close())
                    }
                    crate::terms_cache::TermHolder::TwoPhase(s) => {
                        has_two_phase_commit = true;
                        is_ready_for_commit =
                            is_ready_for_commit && !s.two_phase_commit().borrow().is_waiting();
                        (s.name(), s.stroke(), s.can_close())
                    }
                };

                ui.scope(|ui| {
                    let selectable = ui.selectable_value(
                        current_tab,
                        ChosenTab::TermScreen(idx),
                        if name.is_empty() {
                            "untitled*".to_string()
                        } else if !can_close {
                            name + "*"
                        } else {
                            name
                        },
                    );

                    ui.painter().line_segment(
                        [
                            selectable.rect.left_bottom(),
                            selectable.rect.right_bottom(),
                        ],
                        stroke,
                    );

                    if selectable.secondary_clicked() {
                        close_idx = Some(idx);
                    };
                });
            }
            if let Some(close_idx) = close_idx {
                if let Some(to_be_closed) = self.get_by_idx(close_idx) {
                    let (can_close, name) = match to_be_closed {
                        crate::terms_cache::TermHolder::Normal(s) => (s.can_close(), s.name()),
                        crate::terms_cache::TermHolder::TwoPhase(s) => (s.can_close(), s.name()),
                    };

                    if !can_close {
                        // tab can't be closed - switch to it for the user to see what's going on
                        *current_tab = ChosenTab::TermScreen(close_idx);
                    } else {
                        if self.iter().len() == 1 {
                            *current_tab = ChosenTab::Ask;
                        }
                        if let ChosenTab::TermScreen(idx) = current_tab {
                            if close_idx < *idx {
                                *idx -= 1;
                            } else {
                                *idx = min(*idx, self.iter().len() - 1 - 1);
                            }
                        }

                        self.remove(&name);
                    }
                }
            }
            if has_two_phase_commit {
                let commit_button = egui::Button::new("Finish commit");

                if ui
                    .add_enabled(is_ready_for_commit, commit_button)
                    .on_disabled_hover_text("Still need some approvals")
                    .clicked()
                {
                    output = Some(Output::FinishedCommit);
                }
                // TODO: make this work
                if ui.add_enabled(false, egui::Button::new("Abort")).clicked() {
                    output = Some(Output::AbortedCommit);
                };
            }
        });
        output
    }
}
