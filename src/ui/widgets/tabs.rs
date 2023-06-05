use crate::term_knowledge_base::TermsKnowledgeBase;

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

struct TabsState {
    current_selection: ChoseTabInternal,
    ask: String,
    terms: Vec<TermScreen>,
}

impl Default for TabsState {
    fn default() -> Self {
        Self {
            current_selection: ChoseTabInternal::Ask,
            ask: ASK_TAB_NAME.to_string(),
            terms: vec![],
        }
    }
}

impl TabsState {
    fn push(&mut self, term_screen: TermScreen) {
        self.terms.push(term_screen);
    }

    fn select(&mut self, term_name: &str) {
        if let Some(term_idx) = self.terms.iter().position(|x| x.name() == term_name) {
            self.current_selection = ChoseTabInternal::Term(term_idx);
        }
    }

    fn remove(&mut self, term_name: &str) {
        if let Some(term_idx) = self.terms.iter().position(|x| x.name() == term_name) {
            self.terms.remove(term_idx);
            self.current_selection = ChoseTabInternal::Ask;
        }
    }
}

#[derive(Default)]
pub(crate) struct TermTabs {
    tabs_vec: TabsState,
}

impl TermTabs {
    pub(crate) fn show<'a>(&'a mut self, ui: &mut egui::Ui) -> ChosenTab<'a> {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.tabs_vec.current_selection,
                ChoseTabInternal::Ask,
                egui::RichText::new(ASK_TAB_NAME).strong(),
            );
            ui.separator();

            let mut delete_idx = None;
            for (idx, term) in self.tabs_vec.terms.iter_mut().enumerate() {
                if ui
                    .selectable_value(
                        &mut self.tabs_vec.current_selection,
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
                    delete_idx = Some(idx);
                };
            }
            if let Some(delete_idx) = delete_idx {
                if self.tabs_vec.terms[delete_idx].is_being_edited() {
                    // finish editing before closing a tab
                    self.tabs_vec
                        .select(&self.tabs_vec.terms[delete_idx].name());
                } else {
                    if let ChoseTabInternal::Term(current_idx) = self.tabs_vec.current_selection {
                        if delete_idx == current_idx {
                            self.tabs_vec.current_selection = ChoseTabInternal::Ask;
                        } else if delete_idx < current_idx {
                            self.tabs_vec.current_selection =
                                ChoseTabInternal::Term(current_idx - 1);
                        }
                    }
                    self.tabs_vec.terms.remove(delete_idx);
                }
            }
        });
        match self.tabs_vec.current_selection {
            ChoseTabInternal::Ask => ChosenTab::Ask(&self.tabs_vec.ask),
            ChoseTabInternal::Term(term_screen_idx) => {
                ChosenTab::Term(&mut self.tabs_vec.terms[term_screen_idx])
            }
        }
    }

    pub(crate) fn select<T: TermsKnowledgeBase>(&mut self, term_name: &str, terms: &T) {
        if !self
            .tabs_vec
            .terms
            .iter()
            .any(|screen| screen.name() == term_name)
        {
            self.tabs_vec
                .push(TermScreen::new(&terms.get(&term_name).unwrap().clone()));
        }
        self.tabs_vec.select(term_name);
    }

    pub(crate) fn remove(&mut self, tab_name: &str) {
        self.tabs_vec.remove(tab_name);
    }

    pub(crate) fn add_new_term(&mut self) {
        self.tabs_vec.push(TermScreen::with_new_term());
        self.tabs_vec.select("");
    }
}
