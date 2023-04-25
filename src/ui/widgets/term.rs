use crate::model::fat_term::FatTerm;

pub(crate) fn show(ui: &mut egui::Ui, term: &FatTerm) {
    ui.horizontal(|ui| {
        ui.heading(egui::RichText::new(term.meta.term.name.clone()).strong());
        ui.small_button("edit");
    });
    ui.separator();

    egui::ScrollArea::vertical()
        .id_source("description_scroll_area")
        .show(ui, |ui| {
            ui.with_layout(
                egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                |ui| {
                    ui.label(egui::RichText::new(&term.meta.term.desc).italics());
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
                    for rule in &term.term.rules {
                        // TODO: it might be worth to cache this string
                        let arg_strings: Vec<&str> = rule
                            .arg_bindings
                            .binding
                            .iter()
                            .map(|a| match a {
                                Some(v) => v,
                                None => "_",
                            })
                            .collect();

                        let arguments_string: String = arg_strings.join(", ");

                        let body_strings: Vec<String> = rule
                            .body
                            .iter()
                            .map(|c| {
                                let arg_strings: Vec<&str> = c
                                    .arg_bindings
                                    .binding
                                    .iter()
                                    .map(|a| match a {
                                        Some(v) => v,
                                        None => "_",
                                    })
                                    .collect();

                                let arguments_string: String = arg_strings.join(", ");

                                return format!("{} ( {} )", c.name, arguments_string);
                            })
                            .collect();

                        ui.label(format!(
                            "{} ( {} ) if {}",
                            &term.meta.term.name,
                            arguments_string,
                            body_strings.join(", ")
                        ));
                    }
                    ui.horizontal(|ui| {
                        let mut params = vec![String::new(); term.meta.args.len()];

                        create_rule_placeholder(ui, &term.meta.term.name, params.iter_mut());
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
                    for fact in &term.term.facts {
                        // TODO: it might be worth to cache this string
                        let arg_strings: Vec<&str> = fact
                            .binding
                            .iter()
                            .map(|a| match a {
                                Some(v) => v,
                                None => "_",
                            })
                            .collect();

                        let arguments_string: String = arg_strings.join(", ");
                        ui.label(format!("{} ( {} )", &term.meta.term.name, arguments_string));
                    }
                    let mut params = vec![String::new(); term.meta.args.len()];
                    ui.horizontal(|ui| {
                        create_placeholder(ui, &term.meta.term.name, params.iter_mut());
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
