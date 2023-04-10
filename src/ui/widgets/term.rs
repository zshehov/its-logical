use crate::model::term::Term;

pub(crate) fn show(ui: &mut egui::Ui, term: &Term) {
    ui.horizontal(|ui| {
        ui.heading(egui::RichText::new(term.name.clone()).strong());
        ui.small_button("edit");
    });
    ui.separator();

    egui::ScrollArea::vertical()
        .id_source("description_scroll_area")
        .show(ui, |ui| {
            ui.with_layout(
                egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                |ui| {
                    ui.label(egui::RichText::new(&term.description).italics());
                },
            );
            ui.small_button("edit");
        });
    ui.separator();
    // Rules:
    egui::ScrollArea::vertical()
        .id_source("rules_scroll_area")
        .show(ui, |ui| {
            ui.with_layout(
                egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                |ui| {
                    for (args, rule) in &term.rules {
                        // TODO: it might be worth to cache this string
                        let arguments_string: String = args
                            .iter()
                            .flatten()
                            .cloned()
                            .collect::<Vec<String>>()
                            .join(", ");
                        ui.label(format!(
                            "{} ( {} ) if {}",
                            &term.name, arguments_string, rule
                        ));
                    }
                    ui.horizontal(|ui| {
                        let mut params = vec![String::new(); term.arguments.len()];

                        create_rule_placeholder(ui, &term.name, params.iter_mut());
                        ui.small_button("+");
                    });
                },
            )
        });
    ui.separator();
    // Facts:
    egui::ScrollArea::vertical()
        .id_source("facts_scroll_area")
        .show(ui, |ui| {
            ui.with_layout(
                egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                |ui| {
                    for fact in &term.facts {
                        // TODO: it might be worth to cache this string
                        let arguments_string: String = fact
                            .iter()
                            .flatten()
                            .cloned()
                            .collect::<Vec<String>>()
                            .join(", ");
                        ui.label(format!("{} ( {} )", &term.name, arguments_string));
                    }
                    let mut params = vec![String::new(); term.arguments.len()];
                    ui.horizontal(|ui| {
                        create_placeholder(ui, &term.name, params.iter_mut());
                        ui.small_button("+");
                    });
                },
            )
        });
    ui.separator();
    // Reffered by:
    egui::ScrollArea::vertical()
        .id_source("referred_by_scroll_area")
        .show(ui, |ui| {
            ui.with_layout(
                egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                |ui| {
                    ui.label("Referred by:");
                    ui.label("grandmother");
                },
            )
        });
}

const SINGLE_WIDTH: f32 = 15.0;

// expects to be called in a horizontal layout
fn create_placeholder<'a>(
    ui: &mut egui::Ui,
    term_name: &str,
    parameters: impl Iterator<Item = &'a mut String>,
) {
    ui.label(egui::RichText::new(format!("{} ( ", term_name)).weak());

    let mut added_once = false;
    for param in parameters {
        if added_once {
            ui.label(egui::RichText::new(", ").weak());
        }
        ui.add(
            egui::TextEdit::singleline(param)
                .desired_width(SINGLE_WIDTH)
                .hint_text("X"),
        );
        added_once = true
    }
    ui.label(egui::RichText::new(" )").weak());
}

// expects to be called in a horizontal layout
fn create_rule_placeholder<'a>(
    ui: &mut egui::Ui,
    term_name: &str,
    parameters: impl Iterator<Item = &'a mut String>,
) {
    create_placeholder(ui, term_name, parameters);
    ui.label(egui::RichText::new(" if ").weak());

    // TODO: pass this from the outside
    let mut rule_string = "";
    ui.add(
        egui::TextEdit::singleline(&mut rule_string)
            .desired_width(SINGLE_WIDTH)
            .hint_text("ruuuule"),
    );
}
