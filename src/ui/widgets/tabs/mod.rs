use std::{cell::RefCell, rc::Rc};

use term_tabs::TermTabs;

use crate::{
    model::fat_term::FatTerm,
    term_knowledge_base::{DeleteKnowledgeBase, GetKnowledgeBase, PutKnowledgeBase},
    ui::changes_handling,
};

use commit_tabs::{two_phase_commit::TwoPhaseCommit, CommitTabs};

use super::term_screen::{self, TermScreen};

const ASK_TAB_NAME: &str = "Ask";

#[derive(PartialEq)]
enum ChoseTabInternal {
    Ask,
    TermScreen,
    TwoPhase,
}

pub(crate) mod commit_tabs;
pub(crate) mod term_tabs;

pub(crate) struct Tabs {
    current_selection: ChoseTabInternal,
    ask: String,
    pub(crate) term_tabs: TermTabs<TermScreen>,
    pub(crate) commit_tabs: Option<CommitTabs>,
}

impl Default for Tabs {
    fn default() -> Self {
        Self {
            current_selection: ChoseTabInternal::Ask,
            ask: ASK_TAB_NAME.to_string(),
            term_tabs: TermTabs::new(),
            commit_tabs: None,
        }
    }
}

enum Screens<'a> {
    Ask(&'a String),
    Term(&'a mut TermScreen),
    TwoPhase(Rc<RefCell<TwoPhaseCommit>>),
}

impl Tabs {
    pub(crate) fn push(&mut self, term: &FatTerm) {
        self.term_tabs.push(term)
    }

    pub(crate) fn initiate_two_phase(&mut self) -> &mut CommitTabs {
        // TODO: currently only a single commit is allowed at the same time - maybe provide the
        // ability to have multiple commits at the same time and potentially merge them upon
        // term collision
        self.commit_tabs.get_or_insert(CommitTabs::new())
    }
}

impl Tabs {
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        terms: &mut (impl GetKnowledgeBase + PutKnowledgeBase + DeleteKnowledgeBase),
    ) {
        let chosen_screen = egui::TopBottomPanel::top("tabs_panel")
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let mut chosen_screen = Screens::Ask(&self.ask);
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
            Screens::Ask(_) => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::ui::widgets::ask::show(ui);
                });
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
        terms: &mut (impl GetKnowledgeBase + PutKnowledgeBase + DeleteKnowledgeBase),
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
