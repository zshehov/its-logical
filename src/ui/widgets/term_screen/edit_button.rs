use egui::Ui;

pub(crate) fn show_edit_button(ui: &mut Ui, in_edit: bool) -> bool {
    let toggle_value_text = if in_edit { "💾" } else { "📝" };

    ui.button(egui::RichText::new(toggle_value_text).heading())
        .clicked()
}
