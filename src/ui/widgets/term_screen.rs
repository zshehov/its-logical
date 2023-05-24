use egui::{RichText, TextStyle};

use crate::{
    model::{
        comment::name_description::NameDescription,
        fat_term::FatTerm,
        term::{bound_term::BoundTerm, rule::Rule},
    },
    term_knowledge_base::TermsKnowledgeBase,
};

use super::drag_and_drop::DragAndDrop;
use tracing::debug;

pub(crate) enum Change {
    None,
    TermChange(FatTerm),
}

pub(crate) struct TermScreen {
    term: FatTerm,
    fact_placeholder: Vec<String>,
    rule_placeholder: RulePlaceholder,
    edit_mode: bool,
    changed: bool,
    rules: DragAndDrop<Rule>,
    term_arguments: DragAndDrop<NameDescription>,
}

impl TermScreen {
    pub(crate) fn new(term: &FatTerm) -> Self {
        let term = term.to_owned();
        let args = term.meta.args.to_owned();
        let rules = term.term.rules.to_owned();

        Self {
            term,
            fact_placeholder: vec!["".to_string(); args.len()],
            rule_placeholder: RulePlaceholder::new(args.len()),
            edit_mode: false,
            term_arguments: DragAndDrop::new(args)
                .with_create_item(Box::new(|| NameDescription::new("", ""))),
            changed: false,
            rules: DragAndDrop::new(rules),
        }
    }
    pub(crate) fn with_new_term() -> Self {
        let mut term_arguments =
            DragAndDrop::new(vec![]).with_create_item(Box::new(|| NameDescription::new("", "")));
        term_arguments.unlock();
        let mut rules = DragAndDrop::new(vec![]);
        rules.unlock();
        Self {
            term: FatTerm::default(),
            fact_placeholder: vec![],
            rule_placeholder: RulePlaceholder::new(0),
            edit_mode: true,
            changed: false,
            term_arguments,
            rules,
        }
    }

    pub(crate) fn show<T: TermsKnowledgeBase>(
        &mut self,
        ui: &mut egui::Ui,
        terms_knowledge_base: &T,
    ) -> Change {
        let mut change = Change::None;
        ui.horizontal(|ui| {
            self.changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut self.term.meta.term.name)
                        .clip_text(false)
                        .desired_width(0.0)
                        .hint_text("Enter term name")
                        .frame(self.edit_mode)
                        .interactive(self.edit_mode)
                        .font(TextStyle::Heading),
                )
                .changed();
            let toggle_value_text = if self.edit_mode { "💾" } else { "📝" };
            if ui
                .toggle_value(
                    &mut self.edit_mode,
                    egui::RichText::new(toggle_value_text).heading().monospace(),
                )
                .clicked()
            {
                if !self.edit_mode {
                    let argument_changes = self.term_arguments.lock();
                    let rules_changes = self.rules.lock();
                    self.rule_placeholder = RulePlaceholder::new(self.term_arguments.len());
                    self.fact_placeholder = vec!["".to_string(); self.term_arguments.len()];
                    self.term.meta.args = self.term_arguments.iter().cloned().collect();
                    // TODO: apply argument changes to Rules
                    // TODO: apply argument changes to Facts
                    // TODO: apply argument changes Related
                    if argument_changes.len() > 0 || rules_changes.len() > 0 || self.changed {
                        // TODO: the `self.term` field is probably not needed anyway. The
                        // following is just a hack to syncrhonise between it and the current state
                        // of rules
                        self.term.term.rules = self.rules.iter().cloned().collect();
                        change = Change::TermChange(self.term.clone());
                        debug!("made some changes");
                    }
                } else {
                    self.term_arguments.unlock();
                    self.rule_placeholder.body.unlock();
                    self.rules.unlock();
                }
            }

            self.term_arguments.show(ui, |s, ui| {
                ui.horizontal(|ui| {
                    self.changed |= ui
                        .add(
                            egui::TextEdit::singleline(&mut s.name)
                                .clip_text(false)
                                .hint_text("Enter arg name")
                                .desired_width(0.0)
                                .frame(self.edit_mode)
                                .interactive(self.edit_mode)
                                .font(TextStyle::Body),
                        )
                        .changed();
                    self.changed |= ui
                        .add(
                            egui::TextEdit::singleline(&mut s.desc)
                                .clip_text(false)
                                .desired_width(0.0)
                                .frame(self.edit_mode)
                                .interactive(self.edit_mode)
                                .font(TextStyle::Small),
                        )
                        .changed();
                });
            });
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .id_source("description_scroll_area")
            .show(ui, |ui| {
                ui.with_layout(
                    egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                    |ui| {
                        ui.label(RichText::new("Description").small().italics());
                        self.changed |= ui
                            .add(
                                egui::TextEdit::multiline(&mut self.term.meta.term.desc)
                                    .clip_text(false)
                                    .desired_width(0.0)
                                    .desired_rows(1)
                                    .hint_text("Enter description")
                                    .frame(self.edit_mode)
                                    .interactive(self.edit_mode)
                                    .font(TextStyle::Body),
                            )
                            .changed();
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
                        ui.label(RichText::new("Rules").small().italics());
                        self.rules.show(ui, |r, ui| {
                            let arguments_string: String = r.arg_bindings.binding.join(", ");

                            let body_strings: Vec<String> = r
                                .body
                                .iter()
                                .map(|c| {
                                    let arguments_string: String =
                                        c.arg_bindings.binding.join(", ");

                                    return format!("{} ( {} )", c.name, arguments_string);
                                })
                                .collect();

                            ui.label(format!(
                                "{} ( {} ) if {}",
                                &self.term.meta.term.name,
                                arguments_string,
                                body_strings.join(", ")
                            ));
                        });

                        if self.edit_mode {
                            ui.horizontal(|ui| {
                                show_placeholder(
                                    ui,
                                    &self.term.meta.term.name,
                                    self.rule_placeholder.head.iter_mut(),
                                );
                                ui.label(egui::RichText::new("if").weak());

                                self.rule_placeholder.body.show(ui, |s, ui| {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add(
                                                egui::TextEdit::singleline(&mut s.0)
                                                    .clip_text(false)
                                                    .desired_width(0.0),
                                            )
                                            .lost_focus()
                                        {
                                            // TODO: handle the None here
                                            let t = terms_knowledge_base.get(&s.0).unwrap();
                                            s.1 = vec!["".to_string(); t.meta.args.len()];
                                        }
                                        let mut added_once = false;
                                        ui.label(egui::RichText::new("(").weak());
                                        for param in &mut s.1 {
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
                                });

                                if ui.small_button("Add rule").clicked() {
                                    let mut empty_rule_placeholder =
                                        RulePlaceholder::new(self.term_arguments.len());

                                    std::mem::swap(
                                        &mut self.rule_placeholder,
                                        &mut empty_rule_placeholder,
                                    );

                                    let new_rule = extract_rule(empty_rule_placeholder);
                                    self.term.term.rules.push(new_rule);
                                    // reset the rule placeholder
                                    self.changed = true;
                                }
                            });
                        }
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
                        ui.label(RichText::new("Facts").small().italics());
                        for fact in &self.term.term.facts {
                            let arguments_string: String = fact.binding.join(", ");
                            ui.label(format!(
                                "{} ( {} )",
                                &self.term.meta.term.name, arguments_string
                            ));
                        }

                        if self.edit_mode {
                            ui.horizontal(|ui| {
                                show_placeholder(
                                    ui,
                                    &self.term.meta.term.name,
                                    self.fact_placeholder.iter_mut(),
                                );
                                if ui.small_button("+").clicked() {
                                    let mut empty_fact_placeholder =
                                        vec!["".to_string(); self.term_arguments.len()];
                                    // reset the placeholder
                                    std::mem::swap(
                                        &mut empty_fact_placeholder,
                                        &mut self.fact_placeholder,
                                    );

                                    self.term.term.facts.push(
                                        crate::model::term::args_binding::ArgsBinding {
                                            binding: empty_fact_placeholder,
                                        },
                                    );
                                    self.changed = true;
                                }
                            });
                        }
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
    body: DragAndDrop<(String, Vec<String>)>,
}

impl RulePlaceholder {
    fn new(args_count: usize) -> Self {
        Self {
            head: vec!["".to_string(); args_count],
            body: DragAndDrop::new(vec![("".to_string(), vec![])])
                .with_create_item(Box::new(|| ("".to_string(), vec![]))),
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

fn extract_rule(placeholder: RulePlaceholder) -> Rule {
    let head_binding = placeholder.head;

    let body_bindings = placeholder
        .body
        .iter()
        .filter_map(|(name, args)| {
            // TODO: maybe do the check that name is not existing here
            if name == "" {
                return None;
            }

            Some(BoundTerm {
                name: name.to_owned(),
                arg_bindings: crate::model::term::args_binding::ArgsBinding {
                    binding: args.to_owned(),
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
