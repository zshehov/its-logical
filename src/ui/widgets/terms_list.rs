use crate::suggestions::{self, FuzzySuggestions, Suggestions};

pub(crate) struct TermList {
    filter: String,
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
    ) -> Option<String> {
        let scroll_area = egui::ScrollArea::vertical().auto_shrink([false; 2]);

        ui.add(
            egui::TextEdit::singleline(&mut self.filter)
                .frame(true)
                .desired_width(60.0)
                .clip_text(false)
                .hint_text("Search"),
        );
        // TODO: definitely some pagination is needed here - maybe calculate how many terms can fit in
        // teh current list and trunkate the incoming terms iterator to only that many entries + handle
        // the scrolling
        let mut filtered_terms = suggestions::FuzzySuggestions::new(terms.cloned());

        let filtered_terms =
            <FuzzySuggestions as Suggestions<String>>::filter(&mut filtered_terms, &self.filter);

        scroll_area
            .show(ui, |ui| {
                for term_name in filtered_terms {
                    if ui.small_button(&term_name).clicked() {
                        return Some(term_name);
                    }
                }
                None
            })
            .inner
    }
}
