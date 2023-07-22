use egui::{Response, Ui};

pub(crate) trait Suggestion {
    fn value(&self) -> String;
    fn show(self, ui: &mut Ui) -> Response;
}

pub(crate) trait Suggestions {
    type Suggestion: Suggestion;
    type All: Iterator<Item = Self::Suggestion>;

    fn filter(&self, with: &str) -> Self::All;
}

pub(crate) struct SuggestionsPopup {}

impl SuggestionsPopup {
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        id: egui::Id,
        value: &mut String,
        mut edit_box: impl FnMut(&mut Ui, &mut String) -> Response,
        suggestions: &impl Suggestions,
    ) -> Response {
        let mut response = edit_box(ui, value);

        if response.gained_focus() {
            ui.memory_mut(|m| m.open_popup(id));
        }
        let mut changed = false;

        egui::popup_below_widget(ui, id, &response, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // TODO: cache the filtered response
                for s in suggestions.filter(value) {
                    let suggestion = s.value();

                    if s.show(ui).clicked() {
                        *value = suggestion;
                        changed = true;
                        ui.memory_mut(|m| m.close_popup());
                        response.request_focus();
                        break;
                    }
                }
            });
        });
        if changed {
            response.mark_changed();
        }
        response
    }
}
