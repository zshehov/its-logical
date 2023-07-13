use egui::{RichText, TextStyle};
use tracing::debug;

use crate::{
    changes,
    model::{
        comment::{comment::Comment, name_description::NameDescription},
        fat_term::FatTerm,
        term::{args_binding::ArgsBinding, rule::Rule},
    },
    term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::drag_and_drop::{self, Change, DragAndDrop},
};

use super::placeholder;

pub(crate) enum TermChange {
    Rename,
    DescriptionChange,
    FactsChange,
    ArgRename,
    ArgChanges(Vec<drag_and_drop::Change<NameDescription>>),
    RuleChanges,
}

// Term respresentation that is convenient to use in the TermScreenPIT
struct Term {
    meta: NameDescription,
    rules: DragAndDrop<Rule>,
    facts: DragAndDrop<ArgsBinding>,
    arguments: DragAndDrop<NameDescription>,
    related: Vec<String>,
}

pub(crate) struct TermScreenPIT {
    original_term_name: String,
    term: Term,
    fact_placeholder: placeholder::FactPlaceholder,
    fact_editing: Option<placeholder::FactPlaceholder>,
    rule_placeholder: placeholder::RulePlaceholder,
    rule_editing: Option<placeholder::RulePlaceholder>,
    arg_placeholder: NameDescription,
    arg_rename: bool,
    description_change: bool,
}

impl TermScreenPIT {
    pub(crate) fn name(&self) -> String {
        self.term.meta.name.clone()
    }

    pub(crate) fn extract_term(&self) -> FatTerm {
        (&self.term).into()
    }

    pub(crate) fn new(term: &FatTerm) -> Self {
        let term: Term = term.into();
        let original_name = term.meta.name.clone();

        Self {
            term,
            original_term_name: original_name,
            fact_placeholder: placeholder::FactPlaceholder::new(&[]),
            rule_placeholder: placeholder::RulePlaceholder::new(),
            arg_placeholder: NameDescription::new("", ""),
            arg_rename: false,
            description_change: false,
            fact_editing: None,
            rule_editing: None,
        }
    }

    pub(crate) fn start_changes(&mut self) {
        self.original_term_name = self.term.meta.name.clone();
        self.term.arguments.unlock();
        self.rule_placeholder.unlock();
        self.term.rules.unlock();
        self.term.facts.unlock();
    }

    pub(crate) fn finish_changes(&mut self) -> Option<(Vec<TermChange>, FatTerm)> {
        let mut result = None;
        let mut changes = vec![];

        if self.original_term_name != self.term.meta.name {
            changes.push(TermChange::Rename);
        }
        if self.arg_rename {
            changes.push(TermChange::ArgRename);
            self.arg_rename = false;
        }
        if self.description_change {
            changes.push(TermChange::DescriptionChange);
            self.description_change = false;
        }
        let facts_changes = self.term.facts.lock();
        if !facts_changes.is_empty() {
            changes.push(TermChange::FactsChange);
        }

        let argument_changes = self.term.arguments.lock();
        if !argument_changes.is_empty() {
            changes.push(TermChange::ArgChanges(argument_changes));
        }

        let rules_changes = self.term.rules.lock();
        if !rules_changes.is_empty() {
            changes.push(TermChange::RuleChanges);
        }

        self.rule_placeholder = placeholder::RulePlaceholder::new();
        self.fact_placeholder = placeholder::FactPlaceholder::new(&[]);
        self.arg_placeholder = NameDescription::new("", "");

        if !changes.is_empty() {
            let updated_term = self.extract_term();

            let mut original_name = self.term.meta.name.clone();
            if self.original_term_name.is_empty() {
                // maybe the "new term" case can be handled more gracefully than this
                // if
                self.original_term_name = self.term.meta.name.clone();
            }

            std::mem::swap(&mut original_name, &mut self.original_term_name);

            debug!("made some changes");
            result = Some((changes, updated_term));
        }
        result
    }
}

impl TermScreenPIT {
    pub(crate) fn show<T: TermsKnowledgeBase>(
        &mut self,
        ui: &mut egui::Ui,
        terms_knowledge_base: &T,
        edit_mode: bool,
    ) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.term.meta.name)
                    .clip_text(false)
                    .desired_width(120.0)
                    .hint_text("Term name")
                    .frame(edit_mode)
                    .interactive(edit_mode)
                    .font(TextStyle::Heading),
            );

            ui.vertical(|ui| {
                let mut args_change = self.term.arguments.show(ui, |s, ui| {
                    ui.horizontal(|ui| {
                        self.arg_rename |= Self::show_arg(ui, &mut s.name, &mut s.desc, edit_mode);
                    });
                });

                if edit_mode {
                    ui.horizontal(|ui| {
                        Self::show_arg(
                            ui,
                            &mut self.arg_placeholder.name,
                            &mut self.arg_placeholder.desc,
                            edit_mode,
                        );
                        if ui.small_button("+").clicked() {
                            let mut empty_arg_placeholder = NameDescription::new("", "");
                            // reset the arg placeholder
                            std::mem::swap(&mut self.arg_placeholder, &mut empty_arg_placeholder);

                            self.term.arguments.push(empty_arg_placeholder.clone());
                            args_change.get_or_insert(drag_and_drop::Change::Pushed(
                                empty_arg_placeholder,
                            ));
                        }
                    });
                }
                if let Some(args_change) = args_change {
                    apply_head_args_change(
                        self.term.rules.iter_mut(),
                        self.term.facts.iter_mut(),
                        (&args_change).into(),
                    );
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
                                    .frame(edit_mode)
                                    .interactive(edit_mode)
                                    .font(TextStyle::Body),
                            )
                            .changed();
                    },
                );
            });
        ui.separator();

        self.show_rules_section(ui, edit_mode, terms_knowledge_base);
        ui.separator();

        self.show_facts_section(ui, edit_mode);
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
        if edit_mode {}
    }

    fn show_facts_section(&mut self, ui: &mut egui::Ui, edit_mode: bool) {
        egui::ScrollArea::vertical()
            .id_source("facts_scroll_area")
            .show(ui, |ui| {
                ui.with_layout(
                    egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                    |ui| {
                        ui.label(RichText::new("Facts").small().italics());
                        let mut idx = 0;
                        let mut edited_fact = None;
                        self.term.facts.show(ui, |f, ui| {
                            let arguments_string: String = f.binding.join(", ");
                            ui.label(format!("{} ( {} )", &self.term.meta.name, arguments_string));

                            if edit_mode
                                && self.fact_editing.is_none()
                                && ui.small_button(RichText::new("🖊").monospace()).clicked()
                            {
                                edited_fact = Some(idx);
                            }
                            idx += 1;
                        });

                        if edit_mode {
                            if let Some(edited_fact_idx) = edited_fact {
                                let fact_for_edit = self.term.facts.remove(edited_fact_idx);
                                self.fact_editing =
                                    Some(placeholder::FactPlaceholder::new(&fact_for_edit.binding));
                            }
                            let mut finished_fact_editing = false;
                            if let Some(fact_editing) = &mut self.fact_editing {
                                ui.horizontal(|ui| {
                                    if let Some(edited_fact) = fact_editing.show(
                                        ui,
                                        &self.term.meta.name,
                                        self.term.arguments.iter(),
                                        "✔",
                                    ) {
                                        self.term.facts.push(edited_fact);
                                        finished_fact_editing = true;
                                    }
                                });
                            }
                            if finished_fact_editing {
                                self.fact_editing = None;
                            }
                            ui.horizontal(|ui| {
                                if let Some(new_fact_binding) = self.fact_placeholder.show(
                                    ui,
                                    &self.term.meta.name,
                                    self.term.arguments.iter(),
                                    "Add fact",
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
        edit_mode: bool,
        terms_knowledge_base: &T,
    ) {
        egui::ScrollArea::vertical()
            .id_source("rules_scroll_area")
            .show(ui, |ui| {
                ui.with_layout(
                    egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                    |ui| {
                        ui.label(RichText::new("Rules").small().italics());
                        let mut idx = 0;
                        let mut edited_rule = None;
                        self.term.rules.show(ui, |r, ui| {
                            let arguments_string: String = r.head.binding.join(", ");

                            let body_strings: Vec<String> = r
                                .body
                                .iter()
                                .map(|c| {
                                    let arguments_string: String =
                                        c.arg_bindings.binding.join(", ");

                                    format!("{} ( {} )", c.name, arguments_string)
                                })
                                .collect();

                            ui.label(format!(
                                "{} ( {} ) if {}",
                                &self.term.meta.name,
                                arguments_string,
                                body_strings.join(", ")
                            ));

                            if edit_mode
                                && self.rule_editing.is_none()
                                && ui.small_button(RichText::new("🖊").monospace()).clicked()
                            {
                                edited_rule = Some(idx);
                            }
                            idx += 1;
                        });

                        if edit_mode {
                            if let Some(edited_rule_idx) = edited_rule {
                                let rule_for_edit = self.term.rules.remove(edited_rule_idx);
                                self.rule_editing = Some(rule_for_edit.into());
                            }
                            let mut finished_rule_editing = false;
                            if let Some(rule_editing) = &mut self.rule_editing {
                                ui.horizontal(|ui| {
                                    if let Some(edited_rule) = rule_editing.show(
                                        ui,
                                        &self.term.meta.name,
                                        terms_knowledge_base,
                                        self.term.arguments.iter(),
                                        "✔",
                                    ) {
                                        self.term.rules.push(edited_rule);
                                        finished_rule_editing = true;
                                    }
                                });
                            }
                            if finished_rule_editing {
                                self.rule_editing = None;
                            }

                            ui.horizontal(|ui| {
                                if let Some(new_rule) = self.rule_placeholder.show(
                                    ui,
                                    &self.term.meta.name,
                                    terms_knowledge_base,
                                    self.term.arguments.iter(),
                                    "Add rule",
                                ) {
                                    self.term.rules.push(new_rule);
                                }
                            });
                        }
                    },
                )
            });
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
                term.arguments.iter().as_slice(),
                term.related.as_slice(),
            ),
            crate::model::term::term::Term::new(
                term.facts.iter().as_slice(),
                term.rules.iter().as_slice(),
            ),
        )
    }
}

fn apply_head_args_change<'a>(
    rules: impl Iterator<Item = &'a mut Rule>,
    facts: impl Iterator<Item = &'a mut ArgsBinding>,
    change: changes::ArgsChange,
) {
    for rule in rules {
        let removed_arg =
            changes::propagation::apply_binding_change(&change, &mut rule.head);

        if let Some(removed_arg) = removed_arg {
            for body_term in &mut rule.body {
                for bound_arg in &mut body_term.arg_bindings.binding {
                    if bound_arg == &removed_arg {
                        *bound_arg = "_".to_string();
                    }
                }
            }
        }
    }

    for fact in facts {
        changes::propagation::apply_binding_change(&change, fact);
    }
}

impl From<&drag_and_drop::Change<NameDescription>> for changes::ArgsChange {
    fn from(value: &drag_and_drop::Change<NameDescription>) -> Self {
        match value {
            Change::Pushed(item) => changes::ArgsChange::Pushed(item.name.clone()),
            Change::Moved(moves) => changes::ArgsChange::Moved(moves.clone()),
            Change::Removed(idx) => changes::ArgsChange::Removed(*idx),
        }
    }
}
