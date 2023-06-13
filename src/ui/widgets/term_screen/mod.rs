use egui::RichText;

use crate::model::comment::name_description::NameDescription;
use crate::{
    model::{fat_term::FatTerm, term::rule::Rule},
    term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::drag_and_drop,
};

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

mod edit_button;
mod placeholder;
mod term_screen_pit;

pub(crate) struct TermScreen {
    points_in_time: Vec<term_screen_pit::TermScreenPIT>,
    pit_info: Vec<String>,
    showing_point_in_time: Option<usize>,
    two_phase_commit_initiator: Option<String>,
    current: term_screen_pit::TermScreenPIT,
}

impl TermScreen {
    pub(crate) fn new(term: &FatTerm, in_edit: bool) -> Self {
        Self {
            points_in_time: vec![],
            pit_info: vec![],
            current: term_screen_pit::TermScreenPIT::new(term, in_edit),
            showing_point_in_time: None,
            two_phase_commit_initiator: None,
        }
    }

    pub(crate) fn extract_term(&self) -> FatTerm {
        self.current.extract_term()
    }

    pub(crate) fn with_new_term() -> Self {
        Self {
            points_in_time: vec![],
            pit_info: vec![],
            current: term_screen_pit::TermScreenPIT::with_new_term(),
            showing_point_in_time: None,
            two_phase_commit_initiator: None,
        }
    }

    pub(crate) fn push_pit(&mut self, term: &FatTerm, info: &str) {
        let mut term_screen = term_screen_pit::TermScreenPIT::new(term, false);

        std::mem::swap(&mut self.current, &mut term_screen);
        self.points_in_time.push(term_screen);
        self.pit_info.push(info.to_owned());
    }

    pub(crate) fn name(&self) -> String {
        self.current.name()
    }

    pub(crate) fn is_being_edited(&self) -> bool {
        self.current.is_being_edited()
    }

    pub(crate) fn show<T: TermsKnowledgeBase>(
        &mut self,
        ui: &mut egui::Ui,
        terms_knowledge_base: &T,
    ) -> Option<Change> {
        ui.horizontal(|ui| {
            for (pit_idx, info) in self.pit_info.iter().enumerate() {
                ui.radio_value(
                    &mut self.showing_point_in_time,
                    Some(pit_idx),
                    RichText::new(pit_idx.to_string() + " →").monospace(),
                )
                .on_hover_text(info);
            }
            ui.radio_value(&mut self.showing_point_in_time, None, "");
        });

        if let Some(showing_pit) = self.showing_point_in_time {
            if showing_pit < self.points_in_time.len() {
                // TODO: make sure this is uneditable
                return self.points_in_time[showing_pit].show(ui, terms_knowledge_base, true);
            }
        }
        self.current.show(ui, terms_knowledge_base, false)
    }
}
