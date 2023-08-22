use its_logical::knowledge::model::fat_term::FatTerm;
use its_logical::knowledge::store::{Get, Keys};
use egui::RichText;

use its_logical::changes::change::ArgsChange;

use super::term_screen_pit::{self, TermScreenPIT};

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
    pit_info: Vec<(ChangeSource, Vec<ArgsChange>)>,
}

impl PointsInTime {
    pub(crate) fn new(term: &FatTerm) -> Self {
        Self {
            original: term_screen_pit::TermScreenPIT::new(&term.clone()),
            points_in_time: vec![],
            pit_info: vec![],
        }
    }

    pub(crate) fn push_pit(&mut self, args_changes: &[ArgsChange], term: &FatTerm, source: &str) {
        if source == self.points_in_time.last().unwrap_or(&self.original).name() {
            self.pit_info
                .push((ChangeSource::Internal, args_changes.to_vec()));
        } else {
            self.pit_info.push((
                ChangeSource::External(source.to_owned()),
                args_changes.to_vec(),
            ));
        }

        self.points_in_time
            .push(term_screen_pit::TermScreenPIT::new(term));
    }

    pub(crate) fn iter_mut_pits(&mut self) -> impl Iterator<Item = &mut TermScreenPIT> {
        std::iter::once(&mut self.original).chain(self.points_in_time.iter_mut())
    }

    pub(crate) fn original(&self) -> &term_screen_pit::TermScreenPIT {
        &self.original
    }

    pub(crate) fn latest(&self) -> &term_screen_pit::TermScreenPIT {
        self.points_in_time.last().unwrap_or(&self.original)
    }

    pub(crate) fn accumulated_changes(&self) -> (FatTerm, Vec<ArgsChange>, FatTerm) {
        let accumulated_args_changes = self
            .pit_info
            .iter()
            .flat_map(|(_, args_changes)| args_changes)
            .cloned()
            .collect();
        return (
            self.original().extract_term(),
            accumulated_args_changes,
            self.latest().extract_term(),
        );
    }

    pub(crate) fn len(&self) -> usize {
        self.points_in_time.len() + 1
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
            .on_hover_text(info.0.to_string());
        }
    }

    pub(crate) fn show_pit(
        &mut self,
        ui: &mut egui::Ui,
        showing_pit: usize,
        terms_knowledge_base: &(impl Get + Keys),
    ) {
        if showing_pit == 0 {
            return self.original.show(ui, terms_knowledge_base, false);
        }
        if showing_pit <= self.points_in_time.len() {
            return self.points_in_time[showing_pit - 1].show(ui, terms_knowledge_base, false);
        }
        panic!("[bug] requested unknown point in time")
    }
}
