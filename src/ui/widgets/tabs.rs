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
                            if term.in_edit() {
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
                if self.term_screens[close_idx].in_edit() {
                    // finish editing before closing a tab
                    self.current_selection = ChoseTabInternal::Term(close_idx);
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

    pub(crate) fn push(&mut self, term: &FatTerm) {
        self.term_screens.push(TermScreen::new(term, false));
    }

    pub(crate) fn select_with_push<T: TermsKnowledgeBase>(&mut self, term_name: &str, terms: &T) {
        if !self
            .term_screens
            .iter()
            .any(|screen| screen.name() == term_name)
        {
            self.push(&terms.get(&term_name).unwrap().clone());
        }
        self.select(term_name);
    }

    pub(crate) fn add_new_term(&mut self) {
        self.term_screens.push(TermScreen::with_new_term());
        self.current_selection = ChoseTabInternal::Term(self.term_screens.len() - 1);
    }

    pub(crate) fn borrow_mut(&mut self, names: &[String]) -> Vec<&mut TermScreen> {
        let screens = self
            .term_screens
            .iter_mut()
            .filter(|screen| {
                if names.contains(&screen.name()) {
                    return true;
                }
                false
            })
            .collect();

        screens
    }

    pub(crate) fn get<'a>(&'a self, term_name: &str) -> Option<&'a TermScreen> {
        if let Some(term_idx) = self.term_screens.iter().position(|x| x.name() == term_name) {
            return Some(&self.term_screens[term_idx]);
        }
        None
    }

    pub(crate) fn get_mut<'a>(&'a mut self, term_name: &str) -> Option<&'a mut TermScreen> {
        if let Some(term_idx) = self.term_screens.iter().position(|x| x.name() == term_name) {
            return Some(&mut self.term_screens[term_idx]);
        }
        None
    }

    pub(crate) fn close(&mut self, term_name: &str) {
        if let Some(term_idx) = self.term_screens.iter().position(|x| x.name() == term_name) {
            if let ChoseTabInternal::Term(current_idx) = self.current_selection {
                if term_idx == current_idx {
                    self.current_selection = ChoseTabInternal::Ask;
                }
            }
            self.term_screens.remove(term_idx);
        }
    }

    fn select(&mut self, term_name: &str) {
        if let Some(term_idx) = self.term_screens.iter().position(|x| x.name() == term_name) {
            self.current_selection = ChoseTabInternal::Term(term_idx);
        }
    }
}
