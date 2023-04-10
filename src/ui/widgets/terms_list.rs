use crate::model::term::Term;

pub(crate) fn show<'a>(ui: &mut egui::Ui, terms: impl Iterator<Item = &'a Term>) -> Option<String> {
    let scroll_area = egui::ScrollArea::vertical().auto_shrink([false; 2]);

    scroll_area
        .show(ui, |ui| {
            ui.button(egui::RichText::new("Add term").underline().strong());
            for term in terms {
                if ui.small_button(&term.name).clicked() {
                    return Some(term.name.to_owned());
                }
            }
            None
        })
        .inner
}
