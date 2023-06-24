use egui::RichText;

use crate::{model::fat_term::FatTerm, term_knowledge_base::TermsKnowledgeBase};

use super::{
    term_screen_pit::{self, TermScreenPIT},
    Change, TermScreenError,
};

enum ChangeSource {
    Internal,
    External(String),
}

impl std::fmt::Display for ChangeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeSource::Internal => write!(f, "Internal"),
            ChangeSource::External(s) => write!(f, "{}", s),
        }
    }
}

pub(crate) struct PointsInTime {
    original: term_screen_pit::TermScreenPIT,

    // only relevant during 2-phase-commit
    points_in_time: Vec<term_screen_pit::TermScreenPIT>,
    pit_info: Vec<ChangeSource>,
    is_deleted: bool,
}

impl PointsInTime {
    pub(crate) fn new(term: &FatTerm) -> Self {
        Self {
            original: term_screen_pit::TermScreenPIT::new(&term.clone(), false),
            points_in_time: vec![],
            pit_info: vec![],
            is_deleted: false,
        }
    }

    pub(crate) fn push_pit(&mut self, term: &FatTerm, source: &str) -> Result<(), TermScreenError> {
        if source == self.points_in_time.last().unwrap_or(&self.original).name() {
            self.pit_info.push(ChangeSource::Internal);
        } else {
            self.pit_info
                .push(ChangeSource::External(source.to_owned()));
        }

        self.points_in_time
            .push(term_screen_pit::TermScreenPIT::new(term, false));

        Ok(())
    }

    pub(crate) fn delete(&mut self) {
        self.is_deleted = true
    }

    pub(crate) fn is_deleted(&self) -> bool {
        self.is_deleted
    }

    pub(crate) fn iter_change_sources<'a>(&'a self) -> impl ExactSizeIterator<Item = String> + 'a {
        self.pit_info.iter().map(|x| {
            if let ChangeSource::External(e) = x {
                e.to_owned()
            } else {
                self.original.name()
            }
        })
    }

    pub(crate) fn iter_mut_pits<'a>(
        &'a mut self,
    ) -> impl Iterator<Item = &'a mut TermScreenPIT> + 'a {
        std::iter::once(&mut self.original).chain(self.points_in_time.iter_mut())
    }

    pub(crate) fn original(&self) -> &term_screen_pit::TermScreenPIT {
        &self.original
    }

    pub(crate) fn latest(&self) -> &term_screen_pit::TermScreenPIT {
        self.points_in_time.last().unwrap_or(&self.original)
    }

    pub(crate) fn len(&self) -> usize {
        self.points_in_time.len() + 1
    }

    pub(crate) fn change_origin(&self) -> Option<String> {
        self.pit_info.first().map(|x| match x {
            ChangeSource::Internal => self.original().name(),
            ChangeSource::External(change_source_name) => change_source_name.to_owned(),
        })
    }
}

impl PointsInTime {
    pub(crate) fn show(&self, showing_pit: &mut Option<usize>, ui: &mut egui::Ui) {
        ui.radio_value(showing_pit, Some(0), RichText::new(" →").monospace())
            .on_hover_text("original");
        for (pit_idx, info) in self.pit_info.iter().enumerate() {
            ui.radio_value(
                showing_pit,
                Some(pit_idx + 1),
                RichText::new((pit_idx + 1).to_string() + " →").monospace(),
            )
            .on_hover_text(info.to_string());
        }
    }

    pub(crate) fn show_pit<T: TermsKnowledgeBase>(
        &mut self,
        ui: &mut egui::Ui,
        showing_pit: usize,
        terms_knowledge_base: &T,
    ) -> Option<Change> {
        if self.is_deleted {
            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    ui.label("This Term is to be deleted");
                },
            );
            return None;
        }
        // TODO: make sure these are uneditable
        if showing_pit == 0 {
            return self.original.show(ui, terms_knowledge_base, false, true);
        }
        if showing_pit <= self.points_in_time.len() {
            return self.points_in_time[showing_pit - 1].show(
                ui,
                terms_knowledge_base,
                false,
                true,
            );
        }
        panic!("[bug] requested unknown point in time")
    }
}
