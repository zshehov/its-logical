use crate::knowledge::{
    engine::DummyEngine,
    store::{Delete, Get, Keys, Put},
};
use std::{cell::RefCell, rc::Rc};

use term_tabs::TermTabs;

use crate::{model::fat_term::FatTerm, ui::changes_handling};

use commit_tabs::{two_phase_commit::TwoPhaseCommit, CommitTabs};

use self::ask::Ask;

use super::term_screen::{self, TermScreen};

const ASK_TAB_NAME: &str = "Ask";

#[derive(PartialEq)]
enum ChoseTabInternal {
    Ask,
    TermScreen,
    TwoPhase,
}

pub(crate) mod ask;
pub(crate) mod commit_tabs;
pub(crate) mod term_tabs;

pub(crate) struct Tabs {
    current_selection: ChoseTabInternal,
    ask: ask::Ask,
    pub(crate) term_tabs: TermTabs<TermScreen>,
    pub(crate) commit_tabs: Option<CommitTabs>,
}

impl Default for Tabs {
    fn default() -> Self {
        Self {
            current_selection: ChoseTabInternal::Ask,
            ask: ask::Ask::new(),
            term_tabs: TermTabs::new(),
            commit_tabs: None,
        }
    }
}

enum Screens<'a> {
    Ask(&'a mut Ask),
    Term(&'a mut TermScreen),
    TwoPhase(Rc<RefCell<TwoPhaseCommit>>),
}

impl Tabs {
    pub(crate) fn push(&mut self, term: &FatTerm) {
        self.term_tabs.push(term)
    }
}

impl Tabs {
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        terms: &mut (impl Get + Put + Delete + Keys),
    ) {
        let chosen_screen = egui::TopBottomPanel::top("tabs_panel")
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let mut chosen_screen = Screens::Ask(&mut self.ask);
                    if ui
                        .selectable_value(
                            &mut self.current_selection,
                            ChoseTabInternal::Ask,
                            egui::RichText::new(ASK_TAB_NAME).strong(),
                        )
                        .clicked()
                    {
                        self.term_tabs.unselect();
                        if let Some(commit_screens) = &mut self.commit_tabs {
                            commit_screens.tabs.unselect();
                        }
                    }
                    ui.separator();

                    if let Some(commit_tabs) = &mut self.commit_tabs {
                        if let Some(commit_output) = commit_tabs.show(ui) {
                            match commit_output {
                                commit_tabs::CommitTabsOutput::Selected(screen) => {
                                    self.current_selection = ChoseTabInternal::TwoPhase;
                                    self.term_tabs.unselect();
                                    chosen_screen = Screens::TwoPhase(Rc::clone(screen));
                                }
                                commit_tabs::CommitTabsOutput::FinishedCommit => {
                                    changes_handling::finish_commit(
                                        &mut commit_tabs.tabs,
                                        &mut self.term_tabs,
                                        terms,
                                    );
                                    self.commit_tabs = None;
                                    self.current_selection = ChoseTabInternal::Ask;
                                }
                            }
                        }
                    }
                    if let Some(chosen_term_screen) = self.term_tabs.show(ui) {
                        self.current_selection = ChoseTabInternal::TermScreen;
                        if let Some(commit_screens) = &mut self.commit_tabs {
                            commit_screens.tabs.unselect();
                        }
                        chosen_screen = Screens::Term(chosen_term_screen);
                    }
                    chosen_screen
                })
                .inner
            })
            .inner;

        match chosen_screen {
            Screens::Ask(ask) => {
                egui::CentralPanel::default()
                    .show(ctx, |ui| ask.show(ui, &mut DummyEngine {}, terms));
            }
            Screens::Term(term_screen) => {
                let screen_output = egui::CentralPanel::default()
                    .show(ctx, |ui| term_screen.show(ui, terms))
                    .inner;

                if let Some(screen_output) = screen_output {
                    let original_term = term_screen.extract_term();
                    self.handle_screen_output(&original_term, screen_output, terms);
                }
            }
            Screens::TwoPhase(commit) => {
                let (original_term, screen_output) = {
                    let mut commit = commit.borrow_mut();
                    let screen_output = egui::CentralPanel::default()
                        .show(ctx, |ui| commit.show(ui, terms))
                        .inner;

                    (commit.term.extract_term(), screen_output)
                };

                if let Some(screen_output) = screen_output {
                    self.handle_screen_output(&original_term, screen_output, terms);
                }
            }
        };
    }

    pub(crate) fn select(&mut self, term_name: &str) -> bool {
        if let Some(commit_tab) = &mut self.commit_tabs {
            if commit_tab.tabs.select(term_name) {
                self.current_selection = ChoseTabInternal::TwoPhase;
                self.term_tabs.unselect();
                return true;
            }
        }
        if self.term_tabs.select(term_name) {
            self.current_selection = ChoseTabInternal::TermScreen;
            if let Some(commit_tabs) = &mut self.commit_tabs {
                commit_tabs.tabs.unselect();
            }
            return true;
        }
        false
    }

    fn handle_screen_output(
        &mut self,
        original_term: &FatTerm,
        screen_output: term_screen::Output,
        terms: &mut (impl Get + Put + Delete),
    ) {
        match screen_output {
            term_screen::Output::Changes(changes, updated_term) => {
                changes_handling::handle_changes(
                    self,
                    terms,
                    original_term,
                    &changes,
                    updated_term,
                );
            }
            term_screen::Output::Deleted(_) => {
                changes_handling::handle_deletion(self, terms, original_term);
            }
        }
    }
}
