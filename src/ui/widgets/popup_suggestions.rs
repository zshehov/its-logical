use egui::{Color32, PopupCloseBehavior, Response, Ui, Widget};

use crate::suggestions::Suggestions;

pub(crate) fn show(
    ui: &mut Ui,
    value: &mut String,
    mut edit_box: impl FnMut(&mut Ui, &mut String) -> Response,
    suggestions: &impl Suggestions<String>,
) -> Response {
    let mut response = edit_box(ui, value);
    if response.gained_focus() {
        ui.memory_mut(|m| m.open_popup(response.id));
    }
    let mut changed = false;

    egui::popup_below_widget(
        ui,
        response.id,
        &response,
        PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // TODO: cache the filtered response
                let mut last_lost_focus = false;
                for (idx, s) in suggestions.filter(value).enumerate() {
                    last_lost_focus = false;

                    let suggestion_response =
                        egui::Button::new(&s).fill(Color32::TRANSPARENT).ui(ui);
                    if idx == 0 {
                        // Just ergonimocs - focus is not moved correctly when jumping between the
                        // edit_box and the first suggested element. Forced this here
                        if response.lost_focus() {
                            if ui.input(|i| i.modifiers.shift) {
                                // the edit_box lost focus because of a Shift+Tab, so the
                                // suggestion popup is no longer relevant
                                ui.memory_mut(|m| m.close_popup());
                                break;
                            }
                            suggestion_response.request_focus();
                        } else if suggestion_response.lost_focus()
                            && ui.input(|i| i.modifiers.shift)
                        {
                            response.request_focus();
                        }
                    }
                    if suggestion_response.lost_focus() {
                        last_lost_focus = true;
                    }
                    if suggestion_response.clicked() {
                        *value = s;
                        changed = true;
                        ui.memory_mut(|m| m.close_popup());
                        response.request_focus();
                        break;
                    }
                }
                if last_lost_focus {
                    ui.memory_mut(|m| m.close_popup());
                }
            });
        },
    );
    if changed {
        response.mark_changed();
    }
    response
}
