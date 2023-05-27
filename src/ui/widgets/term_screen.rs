use std::collections::HashSet;

use egui::{Color32, RichText, TextStyle};

use crate::{
    model::{
        comment::{comment::Comment, name_description::NameDescription},
        fat_term::FatTerm,
        term::{args_binding::ArgsBinding, bound_term::BoundTerm, rule::Rule},
    },
    term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::drag_and_drop,
};

use super::drag_and_drop::DragAndDrop;
use tracing::debug;

pub(crate) enum Change {
    None,
    TermChange(FatTerm),
    DeletedTerm,
}

struct Term {
    meta: NameDescription,
    rules: DragAndDrop<Rule>,
    facts: DragAndDrop<ArgsBinding>,
    arguments: DragAndDrop<NameDescription>,
    related: Vec<String>,
}

pub(crate) struct TermScreen {
    term: Term,
    fact_placeholder: Vec<String>,
    rule_placeholder: RulePlaceholder,
    arg_placeholder: NameDescription,
    delete_confirmation: String,
    edit_mode: bool,
    changed: bool,
}

impl TermScreen {
    pub(crate) fn new(term: &FatTerm) -> Self {
        Self {
            term: term.into(),
            fact_placeholder: vec!["".to_string(); term.meta.args.len()],
            rule_placeholder: RulePlaceholder::new(term.meta.args.len()),
            arg_placeholder: NameDescription::new("", ""),
            edit_mode: false,
            changed: false,
            delete_confirmation: "".to_string(),
        }
    }
    pub(crate) fn with_new_term() -> Self {
        let mut term: Term = (&FatTerm::default()).into();

        term.arguments.unlock();
        term.rules.unlock();
        term.facts.unlock();

        Self {
            term,
            fact_placeholder: vec![],
            rule_placeholder: RulePlaceholder::new(0),
            arg_placeholder: NameDescription::new("", ""),
            edit_mode: true,
            changed: false,
            delete_confirmation: "".to_string(),
        }
    }

    pub(crate) fn show<T: TermsKnowledgeBase>(
        &mut self,
        ui: &mut egui::Ui,
        terms_knowledge_base: &mut T,
    ) -> Change {
        let mut change = Change::None;
        ui.horizontal(|ui| {
            self.changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut self.term.meta.name)
                        .clip_text(false)
                        .desired_width(120.0)
                        .hint_text("Term name")
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
                    let argument_changes = self.term.arguments.lock();
                    let rules_changes = self.term.rules.lock();
                    let facts_changes = self.term.facts.lock();
                    self.rule_placeholder = RulePlaceholder::new(self.term.arguments.len());
                    self.fact_placeholder = vec!["".to_string(); self.term.arguments.len()];
                    self.arg_placeholder = NameDescription::new("", "");
                    // TODO: apply argument changes to Rules
                    // TODO: apply argument changes to Facts
                    // TODO: apply argument changes Related
                    if argument_changes.len() > 0
                        || rules_changes.len() > 0
                        || facts_changes.len() > 0
                        || self.changed
                    {
                        let updated_term: FatTerm = (&self.term).into();

                        self.update_related_terms(terms_knowledge_base, rules_changes);

                        change = Change::TermChange(updated_term);
                        debug!("made some changes");
                    }
                } else {
                    self.delete_confirmation = "".to_string();
                    self.term.arguments.unlock();
                    self.rule_placeholder.body.unlock();
                    self.term.rules.unlock();
                    self.term.facts.unlock();
                }
            }

            ui.vertical(|ui| {
                self.term.arguments.show(ui, |s, ui| {
                    ui.horizontal(|ui| {
                        self.changed |= show_arg(ui, &mut s.name, &mut s.desc, self.edit_mode);
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
                            self.changed = true;
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
                        self.changed |= ui
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
                    terms_knowledge_base.delete(&self.term.meta.name);
                    change = Change::DeletedTerm;
                };
            });
        }
        change
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
                                show_placeholder(
                                    ui,
                                    &self.term.meta.name,
                                    self.fact_placeholder.iter_mut(),
                                );
                                if ui.small_button("+").clicked() {
                                    let mut empty_fact_placeholder =
                                        vec!["".to_string(); self.term.arguments.len()];
                                    // reset the placeholder
                                    std::mem::swap(
                                        &mut empty_fact_placeholder,
                                        &mut self.fact_placeholder,
                                    );

                                    self.term.facts.push(
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
    }

    fn show_rules_section<T: TermsKnowledgeBase>(
        &mut self,
        ui: &mut egui::Ui,
        terms_knowledge_base: &mut T,
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
                                show_placeholder(
                                    ui,
                                    &self.term.meta.name,
                                    self.rule_placeholder.head.iter_mut(),
                                );
                                ui.label(egui::RichText::new("if").weak());

                                let mut term_added_to_body = None;
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
                                            term_added_to_body = Some(t.meta.term.name.to_owned());
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
                                if let Some(term_added_to_body) = term_added_to_body {
                                    self.rule_placeholder
                                        .external_terms
                                        .insert(term_added_to_body);
                                }

                                if ui.small_button("add rule").clicked() {
                                    let mut empty_rule_placeholder =
                                        RulePlaceholder::new(self.term.arguments.len());

                                    // reset the rule placeholder
                                    std::mem::swap(
                                        &mut self.rule_placeholder,
                                        &mut empty_rule_placeholder,
                                    );

                                    let new_rule = extract_rule(empty_rule_placeholder);
                                    self.term.rules.push(new_rule);
                                    self.changed = true;
                                }
                            });
                        }
                    },
                )
            });
    }

    fn update_related_terms<T: TermsKnowledgeBase>(
        &mut self,
        terms_knowledge_base: &mut T,
        rules_changes: Vec<drag_and_drop::Change<Rule>>,
    ) {
        let mut related_terms = HashSet::<String>::new();
        let mut removed_related_terms = HashSet::<String>::new();
        let mut pushed_related_terms = HashSet::<String>::new();

        // grab all the currently related terms
        for rule in self.term.rules.iter() {
            for body_term in &rule.body {
                related_terms.insert(body_term.name.clone());
            }
        }

        // grab all the ones that were just removed from this term
        for rule_change in rules_changes {
            match rule_change {
                drag_and_drop::Change::Pushed(pushed_rule) => {
                    for body_term in pushed_rule.body {
                        pushed_related_terms.insert(body_term.name);
                    }
                }
                drag_and_drop::Change::Removed(_, removed_rule) => {
                    for body_term in removed_rule.body {
                        removed_related_terms.insert(body_term.name);
                    }
                }
                drag_and_drop::Change::Moved(_) => {}
            }
        }

        // the actually removed are the ones that have no instance present in this
        // term any longer
        removed_related_terms.retain(|k| !related_terms.contains(k));

        for new_related_term_name in pushed_related_terms.iter() {
            if let Some(mut new_related_term) = terms_knowledge_base.get(&new_related_term_name) {
                if new_related_term.add_related(&self.term.meta.name) {
                    terms_knowledge_base.edit(&new_related_term_name, &new_related_term);
                }
            }
        }

        for removed_related_term_name in removed_related_terms.iter() {
            if let Some(mut removed_related_term) =
                terms_knowledge_base.get(&removed_related_term_name)
            {
                if removed_related_term.remove_related(&self.term.meta.name) {
                    terms_knowledge_base.edit(&removed_related_term_name, &removed_related_term);
                }
            }
        }
    }
}

struct RulePlaceholder {
    head: Vec<String>,
    body: DragAndDrop<(String, Vec<String>)>,
    external_terms: HashSet<String>,
}

impl RulePlaceholder {
    fn new(args_count: usize) -> Self {
        Self {
            head: vec!["".to_string(); args_count],
            body: DragAndDrop::new(vec![("".to_string(), vec![])])
                .with_create_item(Box::new(|| ("".to_string(), vec![]))),
            external_terms: HashSet::new(),
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
            fat_term.meta.related.to_owned(),
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
