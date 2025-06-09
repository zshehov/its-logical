use egui::Layout;

use crate::suggestions::{self, FuzzySuggestions, Suggestions};

pub(crate) struct TermList {
    filter: String,
}

pub(crate) enum TermListOutput {
    AddTerm(String),
    SelectedTerm(String),
}

impl TermList {
    pub(crate) fn new() -> Self {
        Self {
            filter: String::new(),
        }
    }

    pub(crate) fn show<'a>(
        &mut self,
        ui: &mut egui::Ui,
        terms: impl Iterator<Item = &'a String>,
    ) -> Option<TermListOutput> {
        // TODO: definitely some pagination is needed here - maybe calculate how many terms can fit in
        // the current list and truncate the incoming terms iterator to only that many entries + handle
        // the scrolling
        let filtered_terms = suggestions::FuzzySuggestions::new(terms.cloned());

        let filtered_terms =
            <FuzzySuggestions as Suggestions<String>>::filter(&filtered_terms, &self.filter);

        ui.with_layout(Layout::top_down_justified(egui::Align::LEFT), |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.filter)
                        .frame(true)
                        .clip_text(false)
                        .hint_text("Search by term name"),
                );
            });
            ui.separator();

            let scroll_area = egui::ScrollArea::vertical().auto_shrink([true; 2]);
            scroll_area
                .show(ui, |ui| {
                    let mut result = None;
                    let mut no_matches = true;
                    for term_name in filtered_terms {
                        no_matches = false;
                        if ui.small_button(&term_name).clicked() {
                            result = Some(TermListOutput::SelectedTerm(term_name));
                        }
                    }
                    if no_matches {
                        ui.horizontal(|ui| {
                            let add_button = ui.button(egui::RichText::new("Add ").strong());
                            ui.label(
                                egui::RichText::new(self.filter.clone())
                                    .strong()
                                    .underline(),
                            );
                            if add_button.clicked() {
                                result = Some(TermListOutput::AddTerm(self.filter.clone()));
                                self.filter.clear();
                            }
                        });
                    }
                    result
                })
                .inner
        })
        .inner
    }
}
