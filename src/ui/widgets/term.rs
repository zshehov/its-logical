use crate::{model::fat_term::FatTerm, ui::RulePlaceholderState};

pub(crate) enum Change {
    None,
    NewFact,
    NewRule,
    RuleBodyLostFocus(usize, String),
}

pub(crate) fn show(
    ui: &mut egui::Ui,
    term: &FatTerm,
    fact_placeholder_state: &mut Vec<String>,
    rule_placeholder_state: &mut RulePlaceholderState,
) -> Change {
    let mut change = Change::None;
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
                        if let Some((idx, term_that_lost_focus)) = show_rule_placeholder(
                            ui,
                            &term.meta.term.name,
                            rule_placeholder_state.head.iter_mut(),
                            rule_placeholder_state.body.iter_mut(),
                        ) {
                            change = Change::RuleBodyLostFocus(idx, term_that_lost_focus);
                        }
                        if ui.small_button("+").clicked() {
                            change = Change::NewRule;
                        }
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
                    ui.horizontal(|ui| {
                        show_placeholder(
                            ui,
                            &term.meta.term.name,
                            fact_placeholder_state.iter_mut(),
                        );
                        if ui.small_button("+").clicked() {
                            change = Change::NewFact;
                        }
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
    change
}

// TODO: get this from the framework if possible
const SINGLE_CHAR_WIDTH: f32 = 15.0;
// expects to be called in a horizontal layout
fn show_placeholder<'a>(
    ui: &mut egui::Ui,
    term_name: &str,
    parameters: impl Iterator<Item = &'a mut String>,
) {
    ui.label(egui::RichText::new(format!("{} (", term_name)).weak());

    let mut added_once = false;
    for param in parameters {
        if added_once {
            ui.label(egui::RichText::new(", ").weak());
        }
        ui.add(
            egui::TextEdit::singleline(param)
                .clip_text(false)
                .desired_width(SINGLE_CHAR_WIDTH)
                .hint_text("X"),
        );
        added_once = true
    }
    ui.label(egui::RichText::new(")").weak());
}

// expects to be called in a horizontal layout
fn show_rule_placeholder<'a>(
    ui: &mut egui::Ui,
    term_name: &str,
    parameters: impl Iterator<Item = &'a mut String>,
    body_terms: impl Iterator<Item = &'a mut (String, Vec<String>)>,
) -> Option<(usize, String)> {
    show_placeholder(ui, term_name, parameters);
    ui.label(egui::RichText::new("if").weak());

    let mut term_that_lost_focus: Option<(usize, String)> = None;

    ui.vertical(|ui| {
        for (idx, (name, parameters)) in body_terms.enumerate() {
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::TextEdit::singleline(name)
                            .clip_text(false)
                            .desired_width(0.0),
                    )
                    .lost_focus()
                {
                    term_that_lost_focus = Some((idx, name.clone()));
                }
                let mut added_once = false;
                ui.label(egui::RichText::new("(").weak());
                for param in parameters {
                    if added_once {
                        ui.label(egui::RichText::new(", ").weak());
                    }
                    ui.add(
                        egui::TextEdit::singleline(param)
                            .clip_text(false)
                            .desired_width(SINGLE_CHAR_WIDTH)
                            .hint_text("X"),
                    );
                    added_once = true
                }
                ui.label(egui::RichText::new(")").weak());
            });
        }
    });
    term_that_lost_focus
}
