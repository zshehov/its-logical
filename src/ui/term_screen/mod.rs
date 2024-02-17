use egui::Color32;
use its_logical::knowledge::model::fat_term::FatTerm;
use its_logical::knowledge::store::{Get, Keys};

use self::points_in_time::PointsInTime;
use self::term_screen_pit::{TermChange, TermScreenPIT};

pub(crate) enum Output {
    Changes(Vec<TermChange>, FatTerm),
    Deleted(String),
}

mod edit_button;
mod placeholder;
pub(crate) mod points_in_time;
pub(crate) mod term_screen_pit;

// TermScreen owns the state
// - for the different points in time of a term
pub(crate) struct TermScreen {
    points_in_time: PointsInTime,
    showing_point_in_time: Option<usize>,
    current: Option<term_screen_pit::TermScreenPIT>,
    delete_confirmation: String,
    in_deletion: bool,
}

impl crate::terms_cache::NamedTerm for TermScreen {
    fn new(term: &FatTerm) -> Self {
        Self::new(term, false)
    }

    fn name(&self) -> String {
        self.name()
    }

    fn term(&self) -> FatTerm {
        self.extract_term()
    }
}

impl crate::terms_cache::change_handling::automatic::Apply for TermScreen {
    fn apply(&mut self, f: impl Fn(&FatTerm) -> FatTerm) {
        let (pits, current) = self.get_pits_mut();

        let update_screen = |term_screen: &mut TermScreenPIT| {
            let before = term_screen.extract_term();
            let after = f(&before);

            *term_screen = TermScreenPIT::new(&after);
        };

        pits.iter_mut_pits().for_each(update_screen);
        if let Some(current) = current {
            update_screen(current);
            current.start_changes();
        }
    }
}

// TermScreen behaviour:
// - track all the atomic states of a term due to changes made to it
impl TermScreen {
    pub(crate) fn new(term: &FatTerm, in_edit: bool) -> Self {
        Self {
            points_in_time: PointsInTime::new(term),
            current: if in_edit {
                let mut editing_screen = TermScreenPIT::new(term);
                editing_screen.start_changes();
                Some(editing_screen)
            } else {
                None
            },
            showing_point_in_time: if in_edit { None } else { Some(0) },
            delete_confirmation: "".to_string(),
            in_deletion: false,
        }
    }

    pub(crate) fn choose_pit(&mut self, pit_idx: usize) {
        if pit_idx < self.points_in_time.len() {
            self.showing_point_in_time = Some(pit_idx);
        }
    }

    pub(crate) fn extract_term(&self) -> FatTerm {
        self.points_in_time.latest().extract_term()
    }

    pub(crate) fn is_in_deletion(&self) -> bool {
        self.in_deletion
    }

    pub(crate) fn put_in_deletion(&mut self) {
        self.in_deletion = true;
    }

    pub(crate) fn get_pits(&self) -> &PointsInTime {
        &self.points_in_time
    }

    pub(crate) fn get_pits_mut(&mut self) -> (&mut PointsInTime, Option<&mut TermScreenPIT>) {
        (&mut self.points_in_time, self.current.as_mut())
    }

    pub(crate) fn is_ready_for_change(&self) -> bool {
        if self.current.is_some() {
            return false;
        }
        true
    }

    pub(crate) fn start_changes(&mut self) {
        self.showing_point_in_time = None;
        self.current
            .insert(term_screen_pit::TermScreenPIT::new(
                // TODO: it's a bit weird to extract and recreate, however the current alternative is
                // to perform a heavy (a lot stuff will Derive(Clone)) clone
                &self.points_in_time.latest().extract_term(),
            ))
            .start_changes();
    }

    pub(crate) fn name(&self) -> String {
        self.points_in_time.original().name()
    }

    pub(crate) fn in_edit(&self) -> bool {
        self.current.is_some()
    }
}

impl TermScreen {
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        terms_knowledge_base: &(impl Get + Keys),
    ) -> Option<Output> {
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

        // show the edit/save buttons
        if !self.in_deletion {
            match &mut self.current {
                Some(current) => {
                    if edit_button::show_edit_button(ui, true) {
                        // one last frame of the term not being editable with the newest state
                        current.show(ui, terms_knowledge_base, false);
                        let changes = current.finish_changes();
                        self.current = None;
                        self.showing_point_in_time = Some(self.points_in_time.len() - 1);
                        if let Some((changes, updated_term)) = changes {
                            return Some(Output::Changes(changes, updated_term));
                        }
                    }
                }
                None => {
                    if edit_button::show_edit_button(ui, false) {
                        self.start_changes();
                    }
                }
            };
        } else {
            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    ui.label("This Term is to be deleted");
                },
            );
            return None;
        }

        // show the actual content of the currently shown screen (a pit or the edit screen)
        match self.showing_point_in_time {
            Some(showing_pit) => {
                self.points_in_time
                    .show_pit(ui, showing_pit, terms_knowledge_base);
                None
            }
            None => {
                self.current
                    .as_mut()
                    .expect("current should always be present if a point in time is not chosen")
                    .show(ui, terms_knowledge_base, true);

                ui.separator();
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.delete_confirmation)
                            .clip_text(false)
                            .hint_text("delete")
                            .desired_width(60.0),
                    );
                    let mut delete_button = egui::Button::new("🗑");

                    let deletion_confirmed = self.delete_confirmation == "delete";
                    if deletion_confirmed {
                        delete_button = delete_button.fill(Color32::RED);
                    }
                    if ui
                        .add_enabled(deletion_confirmed, delete_button)
                        .on_disabled_hover_text("Type \"delete\" in the box to the left")
                        .clicked()
                    {
                        self.current = None;
                        self.showing_point_in_time = Some(self.points_in_time.len() - 1);
                        self.in_deletion = true;
                        return Some(Output::Deleted(term_name));
                    };
                    None
                })
                .inner
            }
        }
    }
}
