use super::popup_suggestions;

pub(crate) fn show<'a>(
    ui: &mut egui::Ui,
    terms: impl Iterator<Item = &'a String>,
) -> Option<String> {
    let scroll_area = egui::ScrollArea::vertical().auto_shrink([false; 2]);

    let mut asd = "";
    /*
    ui.add(
        egui::TextEdit::singleline(&mut asd)
            .frame(true)
            .desired_width(60.0)
            .clip_text(false)
            .hint_text("Search"),
    );
    */
    scroll_area
        .show(ui, |ui| {
            for term_name in terms {
                if ui.small_button(term_name).clicked() {
                    return Some(term_name.to_owned());
                }
            }
            None
        })
        .inner
}
