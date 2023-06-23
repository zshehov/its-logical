use std::cell::RefCell;
use std::rc::Rc;

use crate::{model::fat_term::FatTerm, term_knowledge_base::TermsKnowledgeBase};

use self::points_in_time::PointsInTime;
use self::term_screen_pit::TermScreenPIT;
use self::two_phase_commit::TwoPhaseCommit;

pub(crate) enum Change {
    // the sequnce of changes and the resulting FatTerm
    Changes(Vec<term_screen_pit::TermChange>, String, FatTerm),
    // a deletion event
    Deleted(String),
}

pub(crate) enum Output {
    Changed(Change),
    FinishTwoPhaseCommit,
    None,
}

#[derive(Debug)]
pub enum TermScreenError {
    DisconnectedChangeChain,
    TermInEdit,
}

mod edit_button;
mod placeholder;
mod points_in_time;
pub(crate) mod term_screen_pit;
pub(crate) mod two_phase_commit;

// TermScreen owns the state
// - for the different points in time of a term
// - related to 2-phase-commits
pub(crate) struct TermScreen {
    points_in_time: PointsInTime,
    showing_point_in_time: Option<usize>,
    current: Option<term_screen_pit::TermScreenPIT>,
    pub(crate) two_phase_commit: Option<Rc<RefCell<TwoPhaseCommit>>>,
}

// TermScreen behaviour:
// - track all the atomic states of a term due to changes made to it in a single 2-phase-commit
// - track 2-phase-commit
//  - track external term changes caused by changes made to this term (approving/aborting)
impl TermScreen {
    pub(crate) fn new(term: &FatTerm, in_edit: bool) -> Self {
        Self {
            points_in_time: PointsInTime::new(term),
            current: if in_edit {
                Some(TermScreenPIT::new(term, true))
            } else {
                None
            },
            showing_point_in_time: if in_edit { None } else { Some(0) },
            two_phase_commit: None,
        }
    }

    pub(crate) fn with_new_term() -> Self {
        Self {
            points_in_time: PointsInTime::new(&FatTerm::default()),
            current: Some(TermScreenPIT::new(&FatTerm::default(), true)),
            showing_point_in_time: None,
            two_phase_commit: None,
        }
    }

    pub(crate) fn extract_term(&self) -> FatTerm {
        self.points_in_time.latest().extract_term()
    }

    pub(crate) fn get_pits(&self) -> &PointsInTime {
        &self.points_in_time
    }

    // the only way to get mutable access to the Points in time is if there is no ongoing edit on
    // top of them
    pub(crate) fn get_pits_mut(
        &mut self,
    ) -> std::result::Result<&mut PointsInTime, TermScreenError> {
        if self.current.is_some() {
            return Err(TermScreenError::TermInEdit);
        }
        Ok(&mut self.points_in_time)
    }

    pub(crate) fn is_ready_for_change(&self, origin: &str) -> bool {
        if self.current.is_some() {
            return false;
        }
        if let Some(current_change_origin) = self.points_in_time.change_origin() {
            if current_change_origin != origin {
                return false;
            }
        }
        return true;
    }

    pub(crate) fn name(&self) -> String {
        self.points_in_time.original().name()
    }

    pub(crate) fn in_edit(&self) -> bool {
        self.current.is_some()
    }
}

impl TermScreen {
    pub(crate) fn show<T: TermsKnowledgeBase>(
        &mut self,
        ui: &mut egui::Ui,
        terms_knowledge_base: &T,
    ) -> Output {
        // show points in time
        if self.points_in_time.len() > 1 || self.in_edit() {
            ui.horizontal(|ui| {
                self.points_in_time
                    .show(&mut self.showing_point_in_time, ui);
                if self.in_edit() {
                    ui.radio_value(&mut self.showing_point_in_time, None, "")
                        .on_hover_text("editing");
                }
            });
        }

        let term_name = self.name();
        // if this term is a part of a 2-phase-commit and should approve a change show the approve
        // button
        if let Some(two_phase_commit) = &mut self.two_phase_commit {
            let mut two_phase_commit = two_phase_commit.borrow_mut();

            if !two_phase_commit.waits_for_approval() {
                if two_phase_commit.is_being_waited() {
                    if ui.button("approve").clicked() {
                        two_phase_commit.approve_all(&term_name);
                    }
                } else if two_phase_commit.is_initiator() {
                    let approved_by = two_phase_commit
                        .iter_approved()
                        .collect::<Vec<String>>()
                        .join(",");

                    if ui
                        .button("finish commit")
                        .on_hover_text("Approved by: ".to_string() + &approved_by)
                        .clicked()
                    {
                        return Output::FinishTwoPhaseCommit;
                    }
                }
            }
        }

        // show the edit/save buttons
        match &mut self.current {
            Some(current) => {
                if edit_button::show_edit_button(ui, true) {
                    // one last frame of the term not being editable with the newest state
                    current.show(ui, terms_knowledge_base, false, false);
                    let changes = current.finish_changes();
                    self.current = None;
                    self.showing_point_in_time = Some(self.points_in_time.len() - 1);
                    if let Some(changes) = changes {
                        return Output::Changed(changes);
                    }
                }
            }
            None => {
                if edit_button::show_edit_button(ui, false) {
                    self.showing_point_in_time = None;
                    self.current.insert(term_screen_pit::TermScreenPIT::new(
                        // TODO: it's a bit weird to extract and recreate, however the current alternative is
                        // to perform a heavy (a lot stuff will Derive(Clone)) clone
                        &self.points_in_time.latest().extract_term(),
                        true,
                    ));
                }
            }
        };

        // show the actual content of the currently shown screen (a pit or the edit screen)
        match self.showing_point_in_time {
            Some(showing_pit) => {
                self.points_in_time
                    .show_pit(ui, showing_pit, terms_knowledge_base);
            }
            None => {
                if let Some(changes) = self
                    .current
                    .as_mut()
                    .expect("current should always be present if a point in time is not chosen")
                    .show(ui, terms_knowledge_base, true, false)
                {
                    return Output::Changed(changes);
                }
            }
        }
        Output::None
    }
}
