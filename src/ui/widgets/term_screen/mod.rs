use std::collections::HashSet;

use egui::{Color32, RichText, TextStyle};

use crate::{
    model::{
        comment::{comment::Comment, name_description::NameDescription},
        fat_term::FatTerm,
        term::{args_binding::ArgsBinding, rule::Rule},
    },
    term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::drag_and_drop,
};

use super::drag_and_drop::DragAndDrop;
use tracing::debug;

pub(crate) enum Change {
    DescriptionChange,
    FactsChange,
    ArgRename,
    ArgChanges(Vec<drag_and_drop::Change<NameDescription>>),
    RuleChanges(Vec<drag_and_drop::Change<Rule>>),
}

pub(crate) enum Result {
    // the sequnce of changes and the resulting FatTerm
    Changes(Vec<Change>, String, FatTerm),
    // a deletion event
    Deleted(String),
}

struct Term {
    meta: NameDescription,
    rules: DragAndDrop<Rule>,
    facts: DragAndDrop<ArgsBinding>,
    arguments: DragAndDrop<NameDescription>,
    related: Vec<String>,
}

mod placeholder;

pub(crate) struct TermScreen {
    original_term_name: String,
    term: Term,
    fact_placeholder: placeholder::FactPlaceholder,
    rule_placeholder: placeholder::RulePlaceholder,
    arg_placeholder: NameDescription,
    delete_confirmation: String,
    edit_mode: bool,
    arg_rename: bool,
    description_change: bool,
}

impl TermScreen {
    pub(crate) fn name(&self) -> String {
        self.term.meta.name.clone()
    }

    pub(crate) fn is_being_edited(&self) -> bool {
        self.edit_mode
    }

    pub(crate) fn new(term: &FatTerm) -> Self {
        Self {
            term: term.into(),
            original_term_name: term.meta.term.name.clone(),
            fact_placeholder: placeholder::FactPlaceholder::new(),
            rule_placeholder: placeholder::RulePlaceholder::new(),
            arg_placeholder: NameDescription::new("", ""),
            edit_mode: false,
            delete_confirmation: "".to_string(),
            arg_rename: false,
            description_change: false,
        }
    }

    pub(crate) fn with_new_term() -> Self {
        let mut term: Term = (&FatTerm::default()).into();

        term.arguments.unlock();
        term.rules.unlock();
        term.facts.unlock();

        Self {
            term,
            original_term_name: "".to_string(),
            fact_placeholder: placeholder::FactPlaceholder::new(),
            rule_placeholder: placeholder::RulePlaceholder::new(),
            arg_placeholder: NameDescription::new("", ""),
            edit_mode: true,
            delete_confirmation: "".to_string(),
            arg_rename: false,
            description_change: false,
        }
    }

    pub(crate) fn show<T: TermsKnowledgeBase>(
        &mut self,
        ui: &mut egui::Ui,
        terms_knowledge_base: &T,
    ) -> Option<Result> {
        let mut result = None;
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.term.meta.name)
                    .clip_text(false)
                    .desired_width(120.0)
                    .hint_text("Term name")
                    .frame(self.edit_mode)
                    .interactive(self.edit_mode)
                    .font(TextStyle::Heading),
            );

            let toggle_value_text = if self.edit_mode { "💾" } else { "📝" };
            if ui
                .toggle_value(
                    &mut self.edit_mode,
                    egui::RichText::new(toggle_value_text).heading().monospace(),
                )
                .clicked()
            {
                if !self.edit_mode {
                    let mut changes = vec![];

                    if self.arg_rename {
                        changes.push(Change::ArgRename);
                        self.arg_rename = false;
                    }
                    if self.description_change {
                        changes.push(Change::DescriptionChange);
                        self.description_change = false;
                    }
                    let facts_changes = self.term.facts.lock();
                    if facts_changes.len() > 0 {
                        changes.push(Change::FactsChange);
                    }

                    let argument_changes = self.term.arguments.lock();
                    if argument_changes.len() > 0 {
                        debug!("woah some arrrrr");
                        changes.push(Change::ArgChanges(argument_changes));
                    }

                    let rules_changes = self.term.rules.lock();
                    if rules_changes.len() > 0 {
                        changes.push(Change::RuleChanges(rules_changes));
                    }

                    self.rule_placeholder = placeholder::RulePlaceholder::new();
                    self.fact_placeholder = placeholder::FactPlaceholder::new();
                    self.arg_placeholder = NameDescription::new("", "");

                    if changes.len() > 0 || self.original_term_name != self.term.meta.name {
                        let updated_term: FatTerm = (&self.term).into();

                        let mut original_name = self.term.meta.name.clone();
                        if self.original_term_name == "" {
                            // maybe the "new term" case can be handled more gracefully than this
                            // if
                            self.original_term_name = self.term.meta.name.clone();
                        }

                        std::mem::swap(&mut original_name, &mut self.original_term_name);

                        debug!("made some changes");
                        result = Some(Result::Changes(changes, original_name, updated_term));
                    }
                } else {
                    self.original_term_name = self.term.meta.name.clone();
                    self.delete_confirmation = "".to_string();
                    self.term.arguments.unlock();
                    self.rule_placeholder.unlock();
                    self.term.rules.unlock();
                    self.term.facts.unlock();
                }
            }

            ui.vertical(|ui| {
                self.term.arguments.show(ui, |s, ui| {
                    ui.horizontal(|ui| {
                        self.arg_rename |= show_arg(ui, &mut s.name, &mut s.desc, self.edit_mode);
                    });
                });

                if self.edit_mode {
                    ui.horizontal(|ui| {
                        show_arg(
                            ui,
                            &mut self.arg_placeholder.name,
                            &mut self.arg_placeholder.desc,
                            self.edit_mode,
                        );
                        if ui.small_button("+").clicked() {
                            let mut empty_arg_placeholder = NameDescription::new("", "");
                            // reset the arg placeholder
                            std::mem::swap(&mut self.arg_placeholder, &mut empty_arg_placeholder);

                            self.term.arguments.push(empty_arg_placeholder);
                        }
                    });
                }
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
                        self.description_change |= ui
                            .add(
                                egui::TextEdit::multiline(&mut self.term.meta.desc)
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

        self.show_rules_section(ui, terms_knowledge_base);
        ui.separator();

        self.show_facts_section(ui);
        ui.separator();

        egui::ScrollArea::vertical()
            .id_source("referred_by_scroll_area")
            .show(ui, |ui| {
                ui.with_layout(
                    egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                    |ui| {
                        ui.label(RichText::new("Referred by").small().italics());
                        ui.horizontal(|ui| {
                            for related_term in &self.term.related {
                                ui.label(related_term);
                            }
                        });
                    },
                )
            });
        if self.edit_mode {
            ui.separator();
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.delete_confirmation)
                        .clip_text(false)
                        .hint_text("delete")
                        .desired_width(60.0),
                );
                let mut delete_button = egui::Button::new("🗑");

                let deletion_confirmed = self.delete_confirmation == "delete";
                if deletion_confirmed {
                    delete_button = delete_button.fill(Color32::RED);
                }
                if ui
                    .add_enabled(deletion_confirmed, delete_button)
                    .on_disabled_hover_text("Type \"delete\" in the box to the left")
                    .clicked()
                {
                    //terms_knowledge_base.delete(&self.term.meta.name);
                    result = Some(Result::Deleted(self.original_term_name.clone()));
                };
            });
        }

        result
    }

    fn show_facts_section(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_source("facts_scroll_area")
            .show(ui, |ui| {
                ui.with_layout(
                    egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                    |ui| {
                        ui.label(RichText::new("Facts").small().italics());
                        self.term.facts.show(ui, |f, ui| {
                            let arguments_string: String = f.binding.join(", ");
                            ui.label(format!("{} ( {} )", &self.term.meta.name, arguments_string));
                        });

                        if self.edit_mode {
                            ui.horizontal(|ui| {
                                if let Some(new_fact_binding) = self.fact_placeholder.show(
                                    ui,
                                    &self.term.meta.name,
                                    self.term.arguments.iter(),
                                ) {
                                    self.term.facts.push(new_fact_binding);
                                }
                            });
                        }
                    },
                )
            });
    }

    fn show_rules_section<T: TermsKnowledgeBase>(
        &mut self,
        ui: &mut egui::Ui,
        terms_knowledge_base: &T,
    ) {
        egui::ScrollArea::vertical()
            .id_source("rules_scroll_area")
            .show(ui, |ui| {
                ui.with_layout(
                    egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                    |ui| {
                        ui.label(RichText::new("Rules").small().italics());
                        self.term.rules.show(ui, |r, ui| {
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
                                &self.term.meta.name,
                                arguments_string,
                                body_strings.join(", ")
                            ));
                        });

                        if self.edit_mode {
                            ui.horizontal(|ui| {
                                if let Some(new_rule) = self.rule_placeholder.show(
                                    ui,
                                    &self.term.meta.name,
                                    terms_knowledge_base,
                                    self.term.arguments.iter(),
                                ) {
                                    self.term.rules.push(new_rule);
                                }
                            });
                        }
                    },
                )
            });
    }
}

fn show_arg(
    ui: &mut egui::Ui,
    arg_name: &mut String,
    arg_desc: &mut String,
    edit_mode: bool,
) -> bool {
    // TODO: fix the hardcoded widths
    let mut changed = false;
    changed |= ui
        .add(
            egui::TextEdit::singleline(arg_name)
                .clip_text(false)
                .hint_text("Name")
                .desired_width(60.0)
                .frame(edit_mode)
                .interactive(edit_mode)
                .font(TextStyle::Body),
        )
        .changed();
    changed |= ui
        .add(
            egui::TextEdit::singleline(arg_desc)
                .clip_text(false)
                .hint_text("Description")
                .desired_width(100.0)
                .frame(edit_mode)
                .interactive(edit_mode)
                .font(TextStyle::Small),
        )
        .changed();
    changed
}

impl Term {
    fn new(
        meta: NameDescription,
        rules: DragAndDrop<Rule>,
        facts: DragAndDrop<ArgsBinding>,
        arguments: DragAndDrop<NameDescription>,
        related: Vec<String>,
    ) -> Self {
        Self {
            meta,
            rules,
            facts,
            arguments,
            related,
        }
    }
}

impl From<&FatTerm> for Term {
    fn from(fat_term: &FatTerm) -> Self {
        Self::new(
            NameDescription::new(&fat_term.meta.term.name, &fat_term.meta.term.desc),
            DragAndDrop::new(fat_term.term.rules.to_owned()),
            DragAndDrop::new(fat_term.term.facts.to_owned()),
            DragAndDrop::new(fat_term.meta.args.to_owned()),
            fat_term.meta.referred_by.to_owned(),
        )
    }
}

impl From<&Term> for FatTerm {
    fn from(term: &Term) -> Self {
        Self::new(
            Comment::new(
                term.meta.to_owned(),
                term.arguments.iter().cloned().collect(),
                term.related.clone(),
            ),
            crate::model::term::term::Term::new(
                term.facts.iter().cloned().collect(),
                term.rules.iter().cloned().collect(),
            ),
        )
    }
}
