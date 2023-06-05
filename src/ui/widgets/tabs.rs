use crate::{model::fat_term::FatTerm, term_knowledge_base::TermsKnowledgeBase};

use super::term_screen::TermScreen;

const ASK_TAB_NAME: &str = "Ask";

pub(crate) enum ChosenTab<'a> {
    Term(&'a mut TermScreen),
    Ask(&'a String),
}

#[derive(PartialEq)]
enum ChoseTabInternal {
    Ask,
    Term(usize),
}

pub(crate) struct Tabs {
    current_selection: ChoseTabInternal,
    ask: String,
    term_screens: Vec<TermScreen>,
}

impl Default for Tabs {
    fn default() -> Self {
        Self {
            current_selection: ChoseTabInternal::Ask,
            ask: ASK_TAB_NAME.to_string(),
            term_screens: vec![],
        }
    }
}

impl Tabs {
    pub(crate) fn show<'a>(&'a mut self, ui: &mut egui::Ui) -> ChosenTab<'a> {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.current_selection,
                ChoseTabInternal::Ask,
                egui::RichText::new(ASK_TAB_NAME).strong(),
            );
            ui.separator();

            let mut close_idx = None;
            for (idx, term) in self.term_screens.iter_mut().enumerate() {
                if ui
                    .selectable_value(
                        &mut self.current_selection,
                        ChoseTabInternal::Term(idx),
                        if term.name() == "" {
                            "untitled".to_string()
                        } else {
                            if term.is_being_edited() {
                                term.name() + "*"
                            } else {
                                term.name()
                            }
                        },
                    )
                    .secondary_clicked()
                {
                    close_idx = Some(idx);
                };
            }
            if let Some(close_idx) = close_idx {
                if self.term_screens[close_idx].is_being_edited() {
                    // finish editing before closing a tab
                    self.select(&self.term_screens[close_idx].name());
                } else {
                    if let ChoseTabInternal::Term(current_idx) = self.current_selection {
                        if close_idx == current_idx {
                            self.current_selection = ChoseTabInternal::Ask;
                        } else if close_idx < current_idx {
                            self.current_selection = ChoseTabInternal::Term(current_idx - 1);
                        }
                    }
                    self.term_screens.remove(close_idx);
                }
            }
        });
        match self.current_selection {
            ChoseTabInternal::Ask => ChosenTab::Ask(&self.ask),
            ChoseTabInternal::Term(term_screen_idx) => {
                ChosenTab::Term(&mut self.term_screens[term_screen_idx])
            }
        }
    }

    pub(crate) fn force_open_in_edit(&mut self, term: &FatTerm) {
        if let Some(term_idx) = self
            .term_screens
            .iter()
            .position(|x| x.name() == term.meta.term.name)
        {
            if self.term_screens[term_idx].is_being_edited() {
                // TODO: handle this properly
            } else {
                self.term_screens[term_idx] = TermScreen::new(term, true);
            }
        } else {
            self.term_screens.push(TermScreen::new(term, true));
        }
    }

    pub(crate) fn force_reload<T: TermsKnowledgeBase>(&mut self, term_name: &str, terms: &T) {
        if let Some(term_idx) = self.term_screens.iter().position(|x| x.name() == term_name) {
            if self.term_screens[term_idx].is_being_edited() {
                // TODO: handle this properly
            } else {
                self.term_screens[term_idx] =
                    TermScreen::new(&terms.get(&term_name).unwrap().clone(), false);
            }
        }
    }

    pub(crate) fn select_with_push<T: TermsKnowledgeBase>(&mut self, term_name: &str, terms: &T) {
        if !self
            .term_screens
            .iter()
            .any(|screen| screen.name() == term_name)
        {
            self.term_screens.push(TermScreen::new(
                &terms.get(&term_name).unwrap().clone(),
                false,
            ));
        }
        self.select(term_name);
    }

    pub(crate) fn add_new_term(&mut self) {
        self.term_screens.push(TermScreen::with_new_term());
        self.current_selection = ChoseTabInternal::Term(self.term_screens.len() - 1);
    }

    pub fn select(&mut self, term_name: &str) {
        if let Some(term_idx) = self.term_screens.iter().position(|x| x.name() == term_name) {
            self.current_selection = ChoseTabInternal::Term(term_idx);
        }
    }
}
