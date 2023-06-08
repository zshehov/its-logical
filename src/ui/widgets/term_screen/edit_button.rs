use egui::Ui;

pub(crate) enum EditToggle {
    None,
    ClickedEdit,
    ClickedSave,
}

pub(crate) fn show_edit_button(ui: &mut Ui, in_edit: &mut bool) -> EditToggle {
    let toggle_value_text = if *in_edit { "💾" } else { "📝" };

    if ui
        .toggle_value(in_edit, egui::RichText::new(toggle_value_text).heading())
        .clicked()
    {
        if *in_edit {
            return EditToggle::ClickedEdit;
        }
        return EditToggle::ClickedSave;
    }
    EditToggle::None
}

pub(crate) enum AcceptChangesToggle {
    None,
    ClickedEdit,
    ClickedAccept,
}

pub(crate) fn show_accept_change_button(ui: &mut Ui, in_edit: &mut bool) -> AcceptChangesToggle {
    let toggle_value_text = if *in_edit { "✅" } else { "📝" };

    if ui
        .toggle_value(in_edit, egui::RichText::new(toggle_value_text).heading())
        .clicked()
    {
        if *in_edit {
            return AcceptChangesToggle::ClickedEdit;
        }
        return AcceptChangesToggle::ClickedAccept;
    }
    AcceptChangesToggle::None
}
