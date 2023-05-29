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

pub(crate) struct TabsState {
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
    pub(crate) fn show<'a>(&'a mut self, ui: &mut egui::Ui) -> ChosenTab<'a> {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.current_selection,
                ChoseTabInternal::Ask,
                egui::RichText::new(ASK_TAB_NAME).strong(),
            );
            ui.separator();

            let mut delete_idx = None;
            for (idx, term) in self.terms.iter_mut().enumerate() {
                if ui
                    .selectable_value(
                        &mut self.current_selection,
                        ChoseTabInternal::Term(idx),
                        if term.name() == "" {
                            "untitled".to_string()
                        } else {
                            term.name()
                        },
                    )
                    .secondary_clicked()
                {
                    delete_idx = Some(idx);
                };
            }
            if let Some(delete_idx) = delete_idx {
                if let ChoseTabInternal::Term(current_idx) = self.current_selection {
                    if delete_idx == current_idx {
                        self.current_selection = ChoseTabInternal::Ask;
                    } else if delete_idx < current_idx {
                        self.current_selection = ChoseTabInternal::Term(current_idx - 1);
                    }
                }
                self.terms.remove(delete_idx);
            }
        });
        match self.current_selection {
            ChoseTabInternal::Ask => ChosenTab::Ask(&self.ask),
            ChoseTabInternal::Term(term_screen_idx) => {
                ChosenTab::Term(&mut self.terms[term_screen_idx])
            }
        }
    }

    pub(crate) fn push(&mut self, term_screen: TermScreen) {
        self.terms.push(term_screen);
    }

    pub(crate) fn select(&mut self, term_name: &str) {
        if let Some(term_idx) = self.terms.iter().position(|x| x.name() == term_name) {
            self.current_selection = ChoseTabInternal::Term(term_idx);
        }
    }

    pub(crate) fn remove(&mut self, term_name: &str) {
        if let Some(term_idx) = self.terms.iter().position(|x| x.name() == term_name) {
            self.terms.remove(term_idx);
            self.current_selection = ChoseTabInternal::Ask;
        }
    }
}
