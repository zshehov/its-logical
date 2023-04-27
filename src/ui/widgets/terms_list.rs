pub(crate) fn show<'a>(
    ui: &mut egui::Ui,
    terms: impl Iterator<Item = &'a String>,
) -> Option<String> {
    let scroll_area = egui::ScrollArea::vertical().auto_shrink([false; 2]);

    scroll_area
        .show(ui, |ui| {
            ui.button(egui::RichText::new("Add term").underline().strong());
            for term_name in terms {
                if ui.small_button(term_name).clicked() {
                    return Some(term_name.to_owned());
                }
            }
            None
        })
        .inner
}
