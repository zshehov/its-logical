use std::ops::Deref;

use egui::RichText;

use crate::model::comment::name_description::NameDescription;
use crate::{
    model::{fat_term::FatTerm, term::rule::Rule},
    term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::drag_and_drop,
};

use self::points_in_time::PointsInTime;
use self::term_screen_pit::TermScreenPIT;

pub(crate) enum TermChange {
    DescriptionChange,
    FactsChange,
    ArgRename,
    ArgChanges(Vec<drag_and_drop::Change<NameDescription>>),
    RuleChanges(Vec<drag_and_drop::Change<Rule>>),
}

pub(crate) enum Change {
    // the sequnce of changes and the resulting FatTerm
    Changes(Vec<TermChange>, String, FatTerm),
    // a deletion event
    Deleted(String),
}

enum ChangeSource {
    Internal,
    External(String),
}

#[derive(Debug)]
pub enum TermScreenError {
    DisconnectedChangeChain,
    TermInEdit,
}

mod edit_button;
mod placeholder;
mod points_in_time;
mod term_screen_pit;

// TermScreen owns the state
// - for the different points in time of a term
// - related to 2-phase-commits
pub(crate) struct TermScreen {
    points_in_time: PointsInTime,
    showing_point_in_time: Option<usize>,
    current: Option<term_screen_pit::TermScreenPIT>,
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
        }
    }

    pub(crate) fn with_new_term() -> Self {
        Self {
            points_in_time: PointsInTime::new(&FatTerm::default()),
            current: Some(TermScreenPIT::new(&FatTerm::default(), true)),
            showing_point_in_time: None,
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
    pub(crate) fn get_pits_mut(&mut self) -> Result<&mut PointsInTime, TermScreenError> {
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
    ) -> Option<Change> {
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

        match &mut self.current {
            Some(current) => {
                if edit_button::show_edit_button(ui, true) {
                    // one last frame of the term not being editable with the newest state
                    current.show(ui, terms_knowledge_base, false, false);
                    let changes = current.finish_changes();
                    self.current = None;
                    self.showing_point_in_time = Some(self.points_in_time.len() - 1);
                    return changes;
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

        match self.showing_point_in_time {
            Some(showing_pit) => {
                return self
                    .points_in_time
                    .show_pit(ui, showing_pit, terms_knowledge_base);
            }
            None => self
                .current
                .as_mut()
                .expect("current should always be present if a point in time is not chosen")
                .show(ui, terms_knowledge_base, true, false),
        }
    }
}

impl std::fmt::Display for ChangeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeSource::Internal => write!(f, "Internal"),
            ChangeSource::External(s) => write!(f, "{}", s),
        }
    }
}
