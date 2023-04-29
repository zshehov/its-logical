use egui::TextStyle;

use crate::{
    model::{
        fat_term::FatTerm,
        term::{bound_term::BoundTerm, rule::Rule},
    },
    term_knowledge_base::TermsKnowledgeBase,
};

use super::drag_and_drop::DragAndDrop;

pub(crate) enum Change {
    None,
    TermChange(FatTerm),
}

pub(crate) struct TermScreen {
    term: FatTerm,
    fact_placeholder: Vec<String>,
    rule_placeholder: RulePlaceholder,
    edit_mode: bool,
    drag_and_drop_test: DragAndDrop,
}

impl TermScreen {
    pub(crate) fn new(term: &FatTerm) -> Self {
        Self {
            term: term.to_owned(),
            fact_placeholder: vec![],
            rule_placeholder: RulePlaceholder::new(term.meta.args.len()),
            edit_mode: false,
            drag_and_drop_test: DragAndDrop::new(vec!["woah".to_string(), "second".to_string(), "asdf".to_string(), "rmrrm".to_string()]),
        }
    }
    pub(crate) fn with_new_term() -> Self {
        Self {
            term: FatTerm::default(),
            fact_placeholder: vec![],
            rule_placeholder: RulePlaceholder::new(0),
            edit_mode: true,
            drag_and_drop_test: DragAndDrop::new(vec!["woah".to_string(), "second".to_string()]),
        }
    }

    pub(crate) fn show<T: TermsKnowledgeBase>(
        &mut self,
        ui: &mut egui::Ui,
        terms_knowledge_base: &T,
    ) -> Change {
        let mut change = Change::None;
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.term.meta.term.name)
                    .clip_text(false)
                    .desired_width(0.0)
                    .hint_text("Enter term name")
                    .frame(self.edit_mode)
                    .interactive(self.edit_mode)
                    .font(TextStyle::Heading),
            );

            self.drag_and_drop_test.show(ui);

            let toggle_value_text = if self.edit_mode { "save" } else { "edit" };
            if ui
                .toggle_value(&mut self.edit_mode, toggle_value_text)
                .clicked()
                && !self.edit_mode
            {
                change = Change::TermChange(self.term.clone());
            }
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .id_source("description_scroll_area")
            .show(ui, |ui| {
                ui.with_layout(
                    egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                    |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.term.meta.term.desc)
                                .clip_text(false)
                                .desired_width(0.0)
                                .desired_rows(1)
                                .hint_text("Enter description")
                                .frame(self.edit_mode)
                                .interactive(self.edit_mode)
                                .font(TextStyle::Body),
                        );
                    },
                );
            });
        ui.separator();
        // Rules:
        egui::ScrollArea::vertical()
            .id_source("rules_scroll_area")
            .show(ui, |ui| {
                ui.with_layout(
                    egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                    |ui| {
                        for rule in &self.term.term.rules {
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
                                &self.term.meta.term.name,
                                arguments_string,
                                body_strings.join(", ")
                            ));
                        }
                        ui.horizontal(|ui| {
                            if let Some((idx, term_that_lost_focus)) = show_rule_placeholder(
                                ui,
                                &self.term.meta.term.name,
                                self.rule_placeholder.head.iter_mut(),
                                self.rule_placeholder.body.iter_mut(),
                            ) {
                                // TODO: handle the None here
                                let t = terms_knowledge_base.get(&term_that_lost_focus).unwrap();
                                self.rule_placeholder.body[idx] = (
                                    term_that_lost_focus,
                                    vec!["".to_string(); t.meta.args.len()],
                                );
                                self.rule_placeholder.body.push(("".to_string(), vec![]));
                            }
                            if ui.small_button("+").clicked() {
                                let new_rule = placeholder_rule_to_rule(&self.rule_placeholder);
                                self.term.term.rules.push(new_rule);

                                change = Change::TermChange(self.term.clone());
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
                        for fact in &self.term.term.facts {
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
                            ui.label(format!(
                                "{} ( {} )",
                                &self.term.meta.term.name, arguments_string
                            ));
                        }
                        ui.horizontal(|ui| {
                            show_placeholder(
                                ui,
                                &self.term.meta.term.name,
                                self.fact_placeholder.iter_mut(),
                            );
                            if ui.small_button("+").clicked() {
                                let binding = normalize_input_args(self.fact_placeholder.iter());
                                self.term.term.facts.push(
                                    crate::model::term::args_binding::ArgsBinding { binding },
                                );

                                change = Change::TermChange(self.term.clone());
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
}

struct RulePlaceholder {
    head: Vec<String>,
    body: Vec<(String, Vec<String>)>,
}

impl RulePlaceholder {
    fn new(args_count: usize) -> Self {
        Self {
            head: vec!["".to_string(); args_count],
            body: vec![("".to_string(), vec![]); 1],
        }
    }
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

fn normalize_input_args<'a>(input: impl Iterator<Item = &'a String>) -> Vec<Option<String>> {
    input
        .map(|a| {
            if a == "" {
                return None;
            }
            Some(a.to_string())
        })
        .collect()
}

fn placeholder_rule_to_rule(placeholder: &RulePlaceholder) -> Rule {
    let head_binding = normalize_input_args(placeholder.head.iter());

    let body_bindings = placeholder
        .body
        .iter()
        .filter_map(|(name, args)| {
            // TODO: maybe do the check that name is not existing here
            if name == "" {
                return None;
            }

            let bound_args = normalize_input_args(args.iter());

            Some(BoundTerm {
                name: name.to_owned(),
                arg_bindings: crate::model::term::args_binding::ArgsBinding {
                    binding: bound_args,
                },
            })
        })
        .collect();

    Rule {
        arg_bindings: crate::model::term::args_binding::ArgsBinding {
            binding: head_binding,
        },
        body: body_bindings,
    }
}
