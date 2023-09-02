use its_logical::changes::{self, change};
use its_logical::knowledge::model::fat_term::FatTerm;
use its_logical::knowledge::{
    engine::DummyEngine,
    store::{Delete, Get, Keys, Put},
};

use crate::terms_cache::{TermHolder, TermsCache};

use self::two_phase_commit_screen::TwoPhaseCommitScreen;

use super::term_screen::term_screen_pit::TermChange;
use super::term_screen::{self, TermScreen};

const ASK_TAB_NAME: &str = "Ask";

#[derive(PartialEq)]
pub(crate) enum ChosenTab {
    Ask,
    TermScreen(usize),
}

pub(crate) mod ask;
pub(crate) mod term_tabs;
pub(crate) mod two_phase_commit_screen;

pub(crate) struct Tabs {
    current_selection: ChosenTab,
    ask: ask::Ask,
    term_tabs: TermsCache<TermScreen, TwoPhaseCommitScreen>,
}

impl Default for Tabs {
    fn default() -> Self {
        Self {
            current_selection: ChosenTab::Ask,
            ask: ask::Ask::new(),
            term_tabs: TermsCache::default(),
        }
    }
}

impl Tabs {
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        terms: &mut (impl Get + Put + Delete + Keys),
    ) {
        egui::TopBottomPanel::top("tabs_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.current_selection,
                    ChosenTab::Ask,
                    egui::RichText::new(ASK_TAB_NAME).strong(),
                );

                ui.separator();

                if let Some(tabs_output) = self.term_tabs.show(ui, &mut self.current_selection) {
                    match tabs_output {
                        term_tabs::Output::FinishedCommit => {
                            todo!();
                        }
                        term_tabs::Output::AbortedCommit => todo!(),
                    }
                }
            })
        });

        match self.current_selection {
            ChosenTab::Ask => {
                egui::CentralPanel::default()
                    .show(ctx, |ui| self.ask.show(ui, &mut DummyEngine {}, terms));
            }
            ChosenTab::TermScreen(screen_idx) => {
                if let Some(term_screen) = self.term_tabs.get_by_idx_mut(screen_idx) {
                    let screen_output = egui::CentralPanel::default()
                        .show(ctx, |ui| match term_screen {
                            crate::terms_cache::TermHolder::Normal(term_screen) => {
                                term_screen.show(ui, terms)
                            }
                            crate::terms_cache::TermHolder::TwoPhase(term_screen) => {
                                term_screen.show(ui, terms)
                            }
                        })
                        .inner;

                    if let Some(screen_output) = screen_output {
                        let original_term = match term_screen {
                            crate::terms_cache::TermHolder::Normal(term_screen) => {
                                term_screen.extract_term()
                            }
                            crate::terms_cache::TermHolder::TwoPhase(term_screen) => {
                                term_screen.extract_term()
                            }
                        };
                        match screen_output {
                            term_screen::Output::Changes(changes, updated_term) => {
                                let change = change::Change::new(
                                    original_term.to_owned(),
                                    Into::<Vec<changes::change::ArgsChange>>::into(TermChangeVec(
                                        changes,
                                    ))
                                    .as_slice(),
                                    updated_term,
                                );
                                self.term_tabs.handle_change(terms, &change);
                                // TODO: apply the change in the persistence layer
                            }
                            term_screen::Output::Deleted(_) => {
                                self.term_tabs.handle_deletion(&original_term, terms);
                                self.current_selection = ChosenTab::Ask;
                                // TODO: delete from persistence layer
                            }
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn select(&mut self, term_name: &str) -> bool {
        if let Some(screen_idx) = self.term_tabs.find(term_name) {
            self.current_selection = ChosenTab::TermScreen(screen_idx);
            return true;
        }
        false
    }

    pub(crate) fn push(&mut self, term: &FatTerm) {
        self.term_tabs.push(term);
    }

    pub(crate) fn get_mut(
        &mut self,
        term_name: &str,
    ) -> Option<&mut TermHolder<TermScreen, TwoPhaseCommitScreen>> {
        self.term_tabs.get_mut(term_name)
    }

    fn handle_screen_output(
        &mut self,
        original_term: &FatTerm,
        screen_output: term_screen::Output,
        terms: &mut (impl Get + Put + Delete),
    ) {
    }
}

struct TermChangeVec(Vec<TermChange>);
impl From<TermChangeVec> for Vec<changes::change::ArgsChange> {
    fn from(value: TermChangeVec) -> Self {
        value
            .0
            .iter()
            .find_map(|change| {
                if let TermChange::ArgChanges(arg_changes) = change {
                    return Some(arg_changes.iter().map(|x| x.into()).collect());
                }
                None
            })
            .unwrap_or(vec![])
    }
}
